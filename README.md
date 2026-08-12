# Bycard

Bycard é um fichário digital para acompanhar coleções de cartas. O objetivo é ajudar a identificar com clareza quais cartas já estão no fichário e quais ainda faltam.

Este repositório contém o frontend Next.js, a API Rust/Axum, o PostgreSQL local, migrations, importação transacional, catálogo público fictício, autenticação por sessão, coleções pessoais e controle de quantidades.

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
- o catálogo público é inteiramente fictício e não possui associação com franquias reais.
