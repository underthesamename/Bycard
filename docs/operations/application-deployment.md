# Publicação da aplicação

## Arquitetura inicial

A primeira publicação usa três serviços independentes:

- **Netlify Free** executa o frontend Next.js;
- **Render Free** executa a imagem Docker da API;
- **Neon Free** mantém o PostgreSQL, conforme o runbook de banco.

O navegador acessa somente a origem do frontend. As chamadas para `/api/*` são
encaminhadas pelo rewrite do Next.js para a URL interna configurada em
`API_UPSTREAM_URL`. Assim, o cookie `__Host-bycard_session` continua first-party,
sem atributo `Domain`, e a API ainda exige que mutações de autenticação tragam o
`Origin` exato configurado em `WEB_ORIGIN`.

Essa composição é apropriada para um alpha sem orçamento, não para operação
crítica. A API gratuita do Render dorme após inatividade e pode levar cerca de
um minuto para responder novamente. O sistema de arquivos do serviço é efêmero,
o que não afeta a Bycard porque o estado persistente fica no PostgreSQL. O plano
gratuito do Netlify tem limite mensal rígido; mantenha alertas de uso habilitados.

## Configuração versionada

`render.yaml` define uma única instância Docker na região `virginia`, com
readiness em `/health/ready` e deploy automático desligado. Essa trava é
intencional: migrations precisam terminar antes de uma nova API receber tráfego.
O Render gera `SESSION_HMAC_KEY`; os únicos valores solicitados durante a criação
do Blueprint são:

- `DATABASE_URL`: URL direta do papel `bycard_app`, terminada em
  `sslmode=verify-full`;
- `WEB_ORIGIN`: origem HTTPS pública do Netlify, sem caminho ou barra final.

`netlify.toml` fixa o build a partir da raiz do monorepo e publica a saída
`apps/web/.next`. Configure também no painel do Netlify, somente no contexto
**Production**:

- `APP_ENV`: `production`;
- `API_UPSTREAM_URL`: origem HTTPS pública criada pelo Render, sem caminho ou
  barra final.

Esses valores não são segredos, mas precisam estar disponíveis durante o build e
no runtime das funções Next.js. Se o painel oferecer seleção de escopo, marque
**Builds** e **Functions**. Variáveis declaradas somente em `netlify.toml` não são
expostas às funções em execução. Não copie `DATABASE_URL`, `SESSION_HMAC_KEY` ou
qualquer papel proprietário do PostgreSQL para o Netlify.

## Primeira publicação

1. Provisione o Neon na região AWS US East (N. Virginia), crie os papéis
   restritos e configure os environments do GitHub conforme o runbook de banco.
2. Execute **Database release** a partir da branch `main` e confirme que migration,
   importação e verificação terminaram com sucesso.
3. Crie o projeto Netlify conectado ao repositório, selecione `main` como branch
   de produção e `apps/web` como **Package directory**. Anote a origem
   `https://<site>.netlify.app`. O primeiro build pode falhar de forma segura
   enquanto `API_UPSTREAM_URL` ainda não existir.
4. No Render, crie um Blueprint a partir do `render.yaml`, informe o
   `DATABASE_URL` restrito e use a origem do passo anterior como `WEB_ORIGIN`.
   Não cadastre forma de pagamento nem altere o plano `free`.
5. Aguarde o deploy da API e confirme que `/health/ready` retorna HTTP 200. Anote
   a origem `https://<serviço>.onrender.com`.
6. Cadastre `APP_ENV=production` e essa origem como `API_UPSTREAM_URL` no
   contexto de produção do Netlify, com acesso para Builds e Functions, e repita
   o deploy do último commit de `main`.
7. Desabilite Deploy Previews e branch deploys nesta etapa. Depois da primeira
   publicação válida, use **Lock** na lista de deploys para impedir que um build
   futuro seja publicado antes da migration e da API correspondente.
8. Execute o smoke test documentado abaixo.

Não aceite um deploy se o Render ignorar `/health/ready`: essa rota consulta o
banco e impede que uma API sem schema ou conexão válida receba tráfego.

## Releases seguintes

Use exatamente esta ordem para cada commit aprovado em `main`:

1. execute **Database release** para o commit;
2. publique manualmente o mesmo commit no Render e aguarde readiness;
3. confirme que o build Netlify do mesmo commit terminou e publique-o
   manualmente, mantendo o deploy anterior bloqueado até esse momento;
4. execute o smoke test;
5. confira os logs da API sem registrar cookies, tokens ou URLs de banco.

Se a migration falhar, não publique aplicação alguma. Se a API falhar depois de
uma migration compatível, restaure o deploy anterior da API. Migrations são
forward-only; restauração de banco é reservada para corrupção ou perda de dados,
conforme o runbook de PostgreSQL.

## Smoke test

O script é somente leitura. Ele valida liveness e readiness diretas, consulta o
catálogo diretamente e pelo proxy do frontend e confirma os cabeçalhos mínimos
de segurança do site. As consultas repetem por tempo limitado para absorver o
cold start esperado da API gratuita:

```bash
WEB_ORIGIN=https://<site>.netlify.app \
API_ORIGIN=https://<serviço>.onrender.com \
  ./scripts/smoke-production.sh
```

O mesmo teste pode ser executado pela interface do GitHub no workflow
**Production smoke**, informando as duas origens públicas. O job não acessa
nenhum environment nem recebe secrets de produção.

Depois dessa verificação, valide manualmente cadastro, login, logout, alteração
de quantidade e persistência após novo login usando uma conta exclusiva de
teste. Exclua ou identifique claramente essa conta para não confundi-la com
usuários reais.

## Limites e gatilhos de migração

Revise a arquitetura antes de abrir o alpha além de um grupo pequeno. Migre a API
para uma instância sem suspensão quando o cold start prejudicar login ou
navegação, e migre de plano antes de atingir qualquer limite mensal. O rate limit
de autenticação é local ao processo; ele precisa de armazenamento compartilhado
antes de escalar a API para mais de uma réplica.

Referências: [Blueprints do Render](https://render.com/docs/blueprint-spec),
[limites do Render Free](https://render.com/docs/free),
[Next.js no Netlify](https://docs.netlify.com/frameworks/),
[monorepos no Netlify](https://docs.netlify.com/build/configure-builds/monorepos/),
[variáveis em funções do Netlify](https://docs.netlify.com/build/functions/environment-variables/)
e [deploys bloqueados no Netlify](https://docs.netlify.com/deploy/manage-deploys/manage-deploys-overview/).
