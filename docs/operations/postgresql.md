# PostgreSQL de produção

## Decisão operacional

O banco recomendado para a primeira publicação é PostgreSQL gerenciado no Neon Free. Ele custa US$ 0, não exige cartão e, atualmente, inclui 100 CU-horas mensais, 0,5 GB de armazenamento e restauração de até seis horas ou 1 GB de mudanças. Esses limites servem ao lançamento inicial, não a uma operação crítica ou a crescimento contínuo.

A decisão aceita um risco claro para manter custo zero: o provedor não oferece sete dias de restauração no plano gratuito. O projeto compensa parcialmente essa limitação com um dump lógico diário, criptografado e retido por sete dias no GitHub Actions. O backup lógico tem RPO de até 24 horas e o agendamento do GitHub não é uma garantia de execução.

O projeto continua portável. Migrations, importação e verificação vivem na imagem `operations` do `Dockerfile`; somente a configuração de backup e restauração ponto a ponto depende do provedor.

Use endpoint direto, sem pooler, para migrations e `pg_dump`. A API também usa no máximo cinco conexões e não precisa do pooler nesta etapa. Todas as URLs públicas devem terminar em `sslmode=verify-full`.

## Provisionamento

1. Crie o projeto na região AWS US East (N. Virginia), a mesma definida para a API no Render.
2. Mantenha o plano **Free** e não cadastre uma forma de pagamento.
3. Selecione PostgreSQL 18 e crie o banco `bycard` com o papel proprietário padrão do projeto.
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

Crie o environment `production` no GitHub, restrinja-o à branch `main` e configure:

- secret `DATABASE_URL`: URL direta do papel `bycard_app`;
- secret `DATABASE_MIGRATION_URL`: URL direta do proprietário do banco;
- variable `DATABASE_BACKUP_ROLE`: `bycard_backup`.

A API recebe somente `DATABASE_URL`. Nunca injete `DATABASE_MIGRATION_URL`, a senha de backup ou credenciais do proprietário no serviço web.

O workflow também recusa execução fora de `main`. A restrição do environment é uma segunda camada contra uma branch não revisada acessar a credencial proprietária. Quando houver outro mantenedor disponível, configure aprovação manual para esse environment.

Crie também o environment `production-backup`, limitado à branch `main`, sem aprovação manual, e configure estes secrets:

- `PGHOST`: host do endpoint direto, sem protocolo ou parâmetros;
- `PGPORT`: `5432`;
- `PGDATABASE`: `bycard`;
- `PGUSER`: `bycard_backup`;
- `PGPASSWORD`: senha exclusiva do papel de backup;
- `BACKUP_ENCRYPTION_KEY`: chave aleatória de pelo menos 32 caracteres.

O job fixa `PGSSLMODE=verify-full`, `PGSSLROOTCERT=system` e
`PGCHANNELBINDING=require`. Assim, o cliente PostgreSQL 18 usa as autoridades
certificadoras do sistema, valida o hostname do endpoint e exige channel binding.

Gere a chave fora do repositório e guarde outra cópia em um gerenciador de senhas. Sem ela, os artifacts são irrecuperáveis:

```bash
openssl rand -base64 48
```

Não configure revisores obrigatórios em `production-backup`, pois o job agendado ficaria aguardando aprovação. Em contrapartida, proteja `main` e exija PR antes de merge: qualquer código incorporado nessa branch poderá usar os secrets do backup.

Execute o workflow **Database release** antes de publicar uma nova versão da API. Ele:

1. constrói a imagem operacional a partir do commit selecionado;
2. aplica migrations sob lock do SQLx;
3. reaplica os grants de privilégio mínimo;
4. importa idempotentemente `me05`, `me01`, `sv09` e `sv08.5`;
5. confirma migrations, coleções publicadas e contagens de cartas.

Uma falha interrompe a release antes do deploy da aplicação. O workflow usa concorrência exclusiva e não cancela uma operação de banco já iniciada.

## Backup

Para incidentes recentes, use primeiro a restauração gerenciada pelo Neon Free, limitada a até seis horas ou 1 GB de mudanças. Para incidentes mais antigos, o workflow **Database backup** roda diariamente às 05:23 UTC e mantém até sete dumps lógicos criptografados, cada um por sete dias.

O script `scripts/postgres/backup.sh` cria um dump portátil em formato custom e com checksum SHA-256. `scripts/postgres/encrypt-backup.sh` valida esse checksum e cifra o dump com AES-256 antes do upload. Somente o arquivo `.gpg` e seu checksum deixam o runner; o dump em texto claro é apagado antes da publicação.

Artifacts de repositórios públicos podem ser acessados por terceiros. A segurança depende de uma chave longa, secreta e armazenada fora do GitHub. Nunca publique o dump sem criptografia e não escreva a chave em argumentos, logs ou arquivos versionados.

O workflow limita cada arquivo cifrado a 32 MiB, remove artifacts antigos antes do upload e retém no máximo sete cópias, totalizando até 224 MiB mais metadados. A franquia atual do GitHub Free é de 500 MB e é compartilhada com GitHub Packages. Configure um orçamento rígido de US$ 0 para Actions, com bloqueio ao atingir o limite, e não use runners maiores. Se o dump ultrapassar o limite ou a franquia já estiver ocupada, o backup falhará em vez de expandir silenciosamente o consumo previsto.

Agendamentos podem atrasar ou ser descartados sob carga e são desativados após 60 dias sem atividade em repositórios públicos. Habilite notificações de falha e confira diariamente se o último job terminou com sucesso. Essa solução é adequada ao lançamento sem orçamento; não equivale a backup gerenciado com SLA.

Para criar uma cópia local em uma estação operacional segura:

```bash
set -a
source /caminho/seguro/bycard-backup.env
set +a
BACKUP_DIRECTORY=/caminho/seguro \
  BACKUP_NAME=bycard-manual.dump \
  ./scripts/postgres/backup.sh
read -r -s -p "Chave do backup: " BACKUP_ENCRYPTION_KEY
printf '\n'
export BACKUP_ENCRYPTION_KEY
BACKUP_FILE=/caminho/seguro/bycard-manual.dump \
  ./scripts/postgres/encrypt-backup.sh
unset BACKUP_ENCRYPTION_KEY
```

O arquivo de ambiente segue o contrato de `.env.database-operations.example` e não entra no Git.

## Exercício de restauração

Execute trimestralmente e antes de qualquer migration destrutiva:

1. para um incidente com menos de seis horas, use **Restore** e **Time Travel Assist** no Neon para localizar o instante desejado;
2. restaure em uma branch descartável, sem alterar a branch de produção;
3. configure temporariamente `DATABASE_URL` para essa branch;
4. execute `database-operations verify` com os quatro IDs de catálogo;
5. registre duração, resultado, RPO e RTO;
6. remova a branch descartável após a validação.

Para validar um artifact, baixe e extraia os arquivos `.gpg` e `.gpg.sha256` de um job bem-sucedido. Em uma estação segura, descriptografe sem colocar a chave na linha de comando:

```bash
read -r -s -p "Chave do backup: " BACKUP_ENCRYPTION_KEY
printf '\n'
export BACKUP_ENCRYPTION_KEY
ENCRYPTED_BACKUP_FILE=/caminho/bycard.dump.gpg \
  BACKUP_FILE=/caminho/bycard.dump \
  ./scripts/postgres/decrypt-backup.sh
unset BACKUP_ENCRYPTION_KEY
```

Em seguida, crie outro banco vazio, carregue as variáveis libpq do papel proprietário desse destino e execute:

```bash
BACKUP_FILE=/caminho/bycard.dump ./scripts/postgres/restore-empty-database.sh
```

Os scripts verificam os checksums e a integridade criptográfica. O restore recusa qualquer destino que já contenha tabelas e não possui opção de sobrescrever produção. Registre duração, resultado, RPO e RTO reais do exercício.

## Rollback e migrations

Migrations são forward-only. Faça mudanças incompatíveis pelo padrão expandir/migrar/contrair, mantendo a versão anterior da API funcional durante o rollback. Se uma release falhar depois da migration, restaure primeiro a aplicação anterior; use PITR somente quando houver corrupção ou perda de dados confirmada.

Referências: [segurança de conexão do Neon](https://neon.com/docs/security/security-overview), [limites do Neon Free](https://neon.com/pricing), [backup lógico sem pooler](https://neon.com/docs/import/migrate-from-neon), [cobrança do GitHub Actions](https://docs.github.com/en/billing/concepts/product-billing/github-actions), [retenção de artifacts](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts) e [limitações de agendamento](https://docs.github.com/en/actions/how-tos/troubleshoot-workflows).
