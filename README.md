# Bycard

Bycard é um fichário digital para acompanhar coleções de cartas. O objetivo é ajudar a identificar com clareza quais cartas já estão no fichário e quais ainda faltam.

Este repositório contém o frontend Next.js, a API Rust/Axum, o PostgreSQL local, migrations, importação transacional, catálogo público, autenticação por sessão, coleções pessoais e controle de quantidades.

## Pré-requisitos

- Node.js 24 LTS (o projeto fixa `24.18.0` em `.node-version`);
- pnpm 11.20.0;
- Rust 1.97.1 com `rustfmt` e `clippy`;
- Docker com Docker Compose.

## Configuração

```bash
cp .env.example .env
pnpm install
cargo fetch --locked
```

Os valores de `.env.example` são exclusivos para desenvolvimento local. Não os reutilize em produção.

### Contrato de produção

`.env.production.example` documenta todas as variáveis exigidas no ambiente publicado. Os valores reais devem ser configurados no gestor de segredos da hospedagem, nunca em um arquivo versionado.

- `APP_ENV` deve ser `production`;
- `WEB_ORIGIN` deve conter somente a origem HTTPS pública do frontend;
- `DATABASE_URL` deve usar PostgreSQL com `sslmode=verify-full`;
- `SESSION_HMAC_KEY` deve ser um segredo aleatório com pelo menos 32 bytes;
- `API_UPSTREAM_URL` é usada somente pelo servidor Next.js e deve apontar para a API por HTTPS.

Gere o segredo de sessão fora do repositório e salve o resultado diretamente no gestor de segredos:

```bash
openssl rand -base64 48
```

Antes de iniciar a API, valide as variáveis sem abrir conexão com o banco:

```bash
make check-config
```

O verificador rejeita valores locais conhecidos, origens com caminho ou credenciais, banco sem validação completa do certificado e upstream HTTP em produção.

### Imagem de produção da API

A API possui uma imagem multi-stage independente de provedor, com versões de base fixadas por digest, binários compilados em modo `release` e runtime sem root. O contexto do Docker aceita somente os manifests e fontes Rust necessários, impedindo que `.env`, frontend e arquivos locais sejam enviados ao builder.

```bash
make api-image
docker run --rm \
  --env-file /caminho/seguro/bycard-api.env \
  --read-only \
  --cap-drop ALL \
  --security-opt no-new-privileges \
  --publish 8080:8080 \
  bycard-api:local
```

O processo valida toda a configuração antes de escutar conexões, responde a `SIGTERM` com encerramento gracioso e inclui um healthcheck em `/health/live`. Não inclua o arquivo real de variáveis no contexto do Docker nem no repositório.

## Executar localmente

Inicie o banco:

```bash
make db
```

Em um terminal, inicie a API:

```bash
make dev-api
```

Em outro terminal, inicie o frontend:

```bash
make dev-web
```

Também é possível iniciar API e frontend juntos com `make dev`. Esse comando mantém os dois processos anexados ao terminal.

Para aplicar as migrations e importar as duas coleções fictícias:

```bash
make import-demo
```

Para importar uma ou mais coleções físicas reais da TCGdex, informe os IDs das
coleções. A integração é somente leitura, não exige chave e mantém os dados no
PostgreSQL local:

```bash
make import-tcgdex TCGDEX_SET_IDS="me01 me02"
```

O importador aceita opcionalmente outro arquivo como primeiro argumento:

```bash
cargo run -p bycard-api --bin import-demo-catalog -- caminho/catalog.json
```

## Endereços

- frontend: <http://localhost:3000>;
- API: <http://127.0.0.1:8080>;
- liveness: <http://127.0.0.1:8080/health/live>;
- readiness: <http://127.0.0.1:8080/health/ready>;
- PostgreSQL: `127.0.0.1:5432`.

`/health/live` confirma que o processo da API responde. `/health/ready` também executa uma consulta simples no PostgreSQL.

## Verificação

```bash
make format-check
make lint
make test
pnpm build
cargo build --workspace --locked
```

## Migrations

```bash
make migrate
```

## Segurança da autenticação

- senhas usam Argon2id e nunca são armazenadas ou registradas em texto puro;
- a sessão usa um token opaco em cookie `HttpOnly`, `SameSite=Lax` e `Secure` em produção; somente o HMAC do token é persistido;
- cadastro e login exigem um header `Origin` igual ao frontend configurado;
- operações autenticadas mutáveis obtêm um token CSRF curto por sessão e o enviam em `X-CSRF-Token`; CORS não é tratado como proteção CSRF;
- o rate limit de cadastro e login reside em memória e vale apenas por processo. Uma implantação com várias réplicas precisará mover esse estado para um armazenamento compartilhado.

## Limitações atuais

- holdings representam somente a quantidade total por carta, sem variante, idioma, condição ou notas;
- a integração com providers externos não faz parte desta etapa;
- nenhuma arte oficial é armazenada no repositório;
- o catálogo importado não implica associação com Nintendo, Creatures, Game Freak ou The Pokémon Company.
