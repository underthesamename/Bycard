# PostgreSQL de produção

## Decisão operacional

O banco recomendado para a primeira publicação é PostgreSQL gerenciado no Neon, no plano Launch. O plano gratuito não atende ao requisito de recuperação: oferece somente uma janela curta de restore. Produção deve manter a janela de restauração em sete dias.

O projeto continua portável. Migrations, importação e verificação vivem na imagem `operations` do `Dockerfile`; somente a configuração de backup e restauração ponto a ponto depende do provedor.

Use endpoint direto, sem pooler, para migrations e `pg_dump`. A API também usa no máximo cinco conexões e não precisa do pooler nesta etapa. Todas as URLs públicas devem terminar em `sslmode=verify-full`.

## Provisionamento

1. Crie o projeto na mesma região escolhida para a API.
2. Selecione PostgreSQL 18 e crie o banco `bycard` com o papel proprietário padrão do projeto.
3. Em **Settings → Restore window**, selecione sete dias.
4. Crie os papéis restritos por SQL, não pelo painel. Papéis criados pelo painel do Neon recebem associação a `neon_superuser` e não servem para aplicação ou backup.
5. Defina as senhas interativamente no `psql`, para que não sejam gravadas no histórico:

```sql
CREATE ROLE bycard_app LOGIN;
CREATE ROLE bycard_backup LOGIN;
\password bycard_app
\password bycard_backup
```

O comando de migration concede os acessos depois de aplicar o schema:

- proprietário: DDL e migrations, somente no job de release;
- `bycard_app`: `SELECT`, `INSERT`, `UPDATE` e `DELETE`, sem DDL;
- `bycard_backup`: somente `SELECT`.

## Segredos e release

Crie o environment `production` no GitHub e configure:

- secret `DATABASE_URL`: URL direta do papel `bycard_app`;
- secret `DATABASE_MIGRATION_URL`: URL direta do proprietário do banco;
- variable `DATABASE_BACKUP_ROLE`: `bycard_backup`.

A API recebe somente `DATABASE_URL`. Nunca injete `DATABASE_MIGRATION_URL`, a senha de backup ou credenciais do proprietário no serviço web.

Execute o workflow **Database release** antes de publicar uma nova versão da API. Ele:

1. constrói a imagem operacional a partir do commit selecionado;
2. aplica migrations sob lock do SQLx;
3. reaplica os grants de privilégio mínimo;
4. importa idempotentemente `me05`, `me01`, `sv09` e `sv08.5`;
5. confirma migrations, coleções publicadas e contagens de cartas.

Uma falha interrompe a release antes do deploy da aplicação. O workflow usa concorrência exclusiva e não cancela uma operação de banco já iniciada.

## Backup

O mecanismo primário é a restauração ponto a ponto gerenciada pelo Neon, com janela de sete dias. A meta inicial é RPO menor que cinco minutos e RTO menor que trinta minutos; o primeiro exercício real deve medir esses valores em vez de presumir que foram atingidos.

O script `scripts/postgres/backup.sh` cria um dump lógico portátil, em formato custom, com checksum SHA-256. Ele exige um papel somente leitura e variáveis libpq separadas, evitando senha na linha de comando. O arquivo contém dados pessoais e hashes de senha: armazene-o somente em destino privado, criptografado e com retenção definida. Nunca publique dumps como artifact de um repositório público.

Exemplo em uma estação operacional segura:

```bash
set -a
source /caminho/seguro/bycard-backup.env
set +a
BACKUP_DIRECTORY=/caminho/criptografado ./scripts/postgres/backup.sh
```

O arquivo de ambiente segue o contrato de `.env.database-operations.example` e não entra no Git.

## Exercício de restauração

Execute trimestralmente e antes de qualquer migration destrutiva:

1. use **Restore** e **Time Travel Assist** no Neon para localizar o instante desejado;
2. restaure em uma branch descartável, sem alterar a branch de produção;
3. configure temporariamente `DATABASE_URL` para essa branch;
4. execute `database-operations verify` com os quatro IDs de catálogo;
5. registre duração, resultado, RPO e RTO;
6. remova a branch descartável após a validação.

Para validar um dump lógico, crie outro banco vazio e execute:

```bash
BACKUP_FILE=/caminho/bycard.dump ./scripts/postgres/restore-empty-database.sh
```

O script verifica o checksum e recusa qualquer destino que já contenha tabelas. Ele não possui opção de sobrescrever produção.

## Rollback e migrations

Migrations são forward-only. Faça mudanças incompatíveis pelo padrão expandir/migrar/contrair, mantendo a versão anterior da API funcional durante o rollback. Se uma release falhar depois da migration, restaure primeiro a aplicação anterior; use PITR somente quando houver corrupção ou perda de dados confirmada.

Referências: [segurança de conexão do Neon](https://neon.com/docs/security/security-overview), [planos e janelas de restore](https://neon.com/pricing), [configuração da janela de restore](https://neon.com/docs/manage/projects), [backup lógico sem pooler](https://neon.com/docs/import/migrate-from-neon).
