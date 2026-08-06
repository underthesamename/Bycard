# Bycard — Especificação técnica e de produto

**Status:** documento inicial  
**Versão:** 0.1  
**Última atualização:** 5 de agosto de 2026  
**Objetivo atual:** projeto de portfólio com possibilidade de validação futura como produto

## 1. Resumo executivo

Bycard é um fichário digital para colecionadores de cartas acompanharem suas coleções. O usuário escolhe uma coleção, visualiza as cartas pertencentes a ela, registra o que possui, controla quantidades e identifica automaticamente o que ainda falta.

O produto será apresentado como um guia independente e não comercial para colecionadores de Pokémon TCG. A demonstração deverá continuar funcional com catálogo fictício ou placeholders, enquanto imagens e dados reais dependerão de um provider configurado e dos termos aplicáveis à fonte.

O principal valor do projeto como portfólio é demonstrar:

- definição e controle de escopo;
- interface responsiva com uma experiência visual de fichário;
- backend em Rust;
- modelagem relacional e isolamento entre usuários;
- autenticação por sessões;
- importação e normalização de catálogos;
- tratamento de falhas;
- testes automatizados;
- segurança, observabilidade, documentação e implantação.

## 2. Problema e proposta de valor

Colecionadores que compram pacotes ou produtos lacrados frequentemente recorrem a planilhas, anotações ou memória para acompanhar cartas possuídas e faltantes.

Proposta de valor:

> Escolha uma coleção, registre suas cartas e acompanhe visualmente tudo o que você possui e o que ainda falta.

O sistema deve responder:

- Quais cartas eu possuo?
- Quais ainda faltam?
- Quantas cartas únicas tenho?
- Quantas cópias repetidas possuo?
- Qual é o progresso de cada coleção?
- Quais cartas foram registradas recentemente?

## 3. Posicionamento

Bycard não será apresentado como uma categoria inédita nem como concorrente inicial de aplicativos consolidados.

Posicionamento público:

> Bycard é uma implementação própria de uma plataforma de gerenciamento de coleções, criada para demonstrar modelagem de dados, importação resiliente de catálogos e uma experiência visual inspirada em fichários físicos.

O objetivo primário da primeira versão é provar qualidade de execução. A validação de preferência sobre planilhas exige posteriormente catálogo real, usuários reais e métricas de uso; uma demonstração fictícia, isoladamente, não valida essa hipótese comercial.

## 4. Público-alvo

### 4.1 Persona principal

- Colecionador iniciante.
- Compra pacotes e produtos lacrados.
- Não sabe exatamente quais cartas já possui.
- Quer registrar cartas rapidamente e enxergar as faltantes.
- Usa principalmente o celular.
- Possui baixo conhecimento técnico.

### 4.2 Públicos secundários

- colecionadores experientes;
- pessoas com várias coleções;
- usuários com cartas repetidas;
- pessoas que organizam cartas em fichários físicos.

## 5. Princípios do produto

- Mobile-first.
- Registro de uma carta em poucos toques.
- Privado por padrão.
- Nenhuma dependência de API externa durante o uso normal.
- Dados e regras calculados no backend.
- Informação de posse não transmitida somente por cor.
- Engenharia proporcional ao estágio do produto.
- Funcionalidades futuras não devem aumentar a complexidade do primeiro release.

## 6. Propriedade intelectual e catálogo público

Bycard utilizará marca própria e exibirá de forma visível que é um guia não oficial. O caso de uso Pokémon TCG será explícito por decisão do responsável, sem alegação de associação, autorização ou endosso.

Não deverão ser incorporados ao repositório sem licença verificável:

- artes de cartas oficiais;
- logos, símbolos, fontes ou layouts oficiais reproduzidos como identidade do Bycard;
- imagens obtidas sem contrato de uso conhecido;
- alegações de associação ou autorização não existentes.

Uma licença de software ou de uma base comunitária não deve ser presumida como licença das ilustrações e marcas contidas nos dados.

O caráter gratuito e o aviso “Guia não oficial” não equivalem a autorização. A capacidade de desligar o provider real e preservar uma demonstração funcional é requisito de resiliência e redução de risco, não garantia jurídica.

### 6.1 Modos de catálogo

**Demonstração pública**

- marca Bycard com identificação de guia não oficial;
- catálogo fictício ou placeholders quando não houver fonte real autorizada/configurada;
- ilustrações próprias ou licenciadas;
- funcionamento integral sem serviço externo;
- provider padrão do repositório.

**Integração pessoal opcional**

- ativada por configuração;
- executada somente no backend;
- documentada separadamente;
- condicionada aos termos do fornecedor;
- capaz de fornecer nomes, metadados e URLs de imagens reais conforme seu contrato;
- incapaz de tornar o catálogo público indisponível quando falhar.

## 7. Escopo por marcos

O plano diferencia três níveis para impedir que a visão completa seja tratada como escopo imediato.

### 7.1 Marco 1 — catálogo navegável

Objetivo: provar a fatia vertical entre arquivo, banco, API e interface.

Inclui:

1. Duas coleções fictícias.
2. Dezoito cartas por coleção.
3. Importação idempotente de JSON.
4. Persistência em PostgreSQL.
5. Listagem de coleções pela API.
6. Listagem de cartas pela API.
7. Catálogo visual no frontend.
8. Fichário responsivo somente leitura.
9. Busca por nome ou número.
10. Estados de carregamento, vazio, erro e imagem ausente.

Não inclui autenticação nem posses.

Critério de conclusão:

> Uma instalação limpa importa o catálogo fictício e permite navegar por duas coleções em computador e celular.

### 7.2 MVP de portfólio

Inclui, além do Marco 1:

1. Cadastro.
2. Login e logout.
3. Sessão persistente.
4. Adição e remoção de uma coleção pessoal.
5. Incremento, decremento e remoção da posse de uma carta.
6. Controle de quantidade.
7. Filtros entre todas, possuídas e faltantes.
8. Progresso por cartas únicas.
9. Total de cópias e repetidas.
10. Isolamento entre usuários.
11. Testes críticos.
12. Demonstração publicada.
13. README, screenshots e documentação arquitetural.

### 7.3 Fora do MVP

- preços em tempo real;
- câmera e reconhecimento por imagem;
- inteligência artificial;
- marketplace, pagamentos e integração com lojas;
- chat, amigos e negociações;
- perfis públicos;
- conquistas e notificações;
- múltiplos jogos ativos;
- aplicativo móvel nativo;
- avaliação automática de condição;
- variantes reais;
- múltiplos idiomas nas posses;
- wishlist e cartas para troca;
- modo de abertura de pacote;
- painel analítico avançado;
- recuperação de senha e verificação de e-mail;
- sincronização administrativa por interface.

## 8. Fluxos principais

### 8.1 Primeiro acesso no MVP

1. O visitante cria uma conta.
2. A sessão é criada.
3. O usuário escolhe uma coleção.
4. Abre o fichário.
5. Marca as cartas que possui.
6. O progresso é atualizado.
7. Ao retornar, encontra os dados preservados.

### 8.2 Registro rápido

1. O usuário abre uma coleção.
2. Busca a carta por nome ou número, se necessário.
3. Toca para adicionar uma cópia.
4. A interface aplica resposta otimista apenas quando houver reversão clara em caso de erro.
5. O backend persiste e devolve as estatísticas atualizadas.

### 8.3 Remoção de coleção pessoal

No MVP, remover uma coleção pessoal significa deixar de acompanhá-la e excluir suas posses daquela coleção após confirmação explícita. A operação deve ocorrer em uma transação.

## 9. Telas

### 9.1 Landing page

- proposta do produto;
- demonstração do fichário;
- explicação em três passos;
- chamada para criar conta;
- acesso à demonstração;
- aviso de independência quando necessário.

### 9.2 Cadastro e login

Cadastro:

- nome de exibição;
- e-mail;
- senha;
- confirmação da senha.

Login:

- e-mail;
- senha;
- controle para exibir ou ocultar a senha.

Recuperação e verificação de e-mail não bloqueiam o MVP. Se houver aceite de termos futuramente, a versão aceita e o instante do aceite deverão ser armazenados.

### 9.3 Coleções pessoais e catálogo

- capa ou placeholder;
- nome;
- código;
- quantidade de cartas;
- data de lançamento;
- ação para começar ou parar de acompanhar.

O painel estatístico separado não é obrigatório no MVP. A página inicial autenticada pode listar diretamente as coleções pessoais.

### 9.4 Fichário

- cabeçalho da coleção;
- progresso;
- contagens de obtidas, faltantes, cópias e repetidas;
- busca;
- filtros;
- páginas do fichário;
- indicador discreto de quantidade;
- acesso aos detalhes da carta.

### 9.5 Detalhes da carta

- imagem ou placeholder;
- nome;
- número;
- raridade;
- quantidade possuída;
- ações de adicionar e remover;
- origem do catálogo quando aplicável.

## 10. Experiência do fichário

### 10.1 Desktop

- nove espaços por página;
- três cartas por linha;
- ordem determinada por `sort_order`;
- navegação por página;
- foco visível e operação por teclado.

### 10.2 Celular

- uma página por vez;
- até três colunas quando a largura permitir;
- alvos de toque adequados;
- nenhuma dependência de gestos;
- carregamento progressivo de imagens;
- nenhuma tentativa de reproduzir literalmente um livro aberto quando isso prejudicar o uso.

### 10.3 Estados de posse

- possuída: imagem normal e indicação textual ou iconográfica;
- faltante: opacidade reduzida, mas ainda identificável;
- quantidade: exibida sem cobrir informação importante;
- alterações não podem depender apenas de animação ou cor.

### 10.4 Acessibilidade

- HTML semântico;
- texto alternativo adequado;
- contraste suficiente;
- foco visível;
- navegação por teclado;
- nomes acessíveis em botões;
- suporte a movimento reduzido;
- mensagens de erro associadas aos campos;
- atualização de quantidade anunciada quando necessário por tecnologia assistiva.

## 11. Arquitetura tecnológica

### 11.1 Frontend

- Next.js;
- TypeScript;
- App Router;
- Tailwind CSS;
- componentes acessíveis;
- biblioteca de estado do servidor somente quando sua utilidade justificar a dependência;
- PWA após o MVP.

### 11.2 Backend

- Rust;
- Axum;
- Tokio;
- Serde;
- SQLx;
- Tower;
- tracing;
- PostgreSQL.

As versões exatas deverão ser fixadas no momento do scaffold e registradas nos manifests. Este documento não presume versões atuais sem verificação.

### 11.3 Infraestrutura

- Docker e Docker Compose no desenvolvimento;
- PostgreSQL gerenciado em produção;
- HTTPS obrigatório;
- proxy ou plataforma que exponha frontend e API sob o mesmo site;
- armazenamento local ou externo apenas para imagens próprias necessárias;
- migrações versionadas.

Não serão usados inicialmente Redis, Kafka, RabbitMQ, Elasticsearch, Kubernetes, microsserviços ou GraphQL interno.

## 12. Estilo arquitetural

O backend será um monólito modular.

```text
HTTP → handlers → serviços de aplicação → repositórios → PostgreSQL
                        ↓
                    domínio
```

Regras:

- handlers traduzem HTTP, autenticação, entrada e saída;
- serviços representam casos de uso com regras ou transações;
- o domínio não depende de Axum, SQLx nem providers;
- queries simples não precisam de camadas cerimoniais sem comportamento;
- providers externos passam por normalização;
- erros internos não são devolvidos integralmente ao cliente;
- traits são usados em fronteiras realmente substituíveis, não automaticamente em todo repositório.

Módulos iniciais:

```text
auth
users
catalog
collections
holdings
health
```

O módulo `sync` entra quando a importação deixar de ser apenas uma tarefa local simples.

## 13. Organização prevista do repositório

```text
Bycard/
├── apps/
│   ├── web/
│   │   ├── app/
│   │   ├── components/
│   │   ├── features/
│   │   └── lib/
│   └── api/
│       ├── src/
│       │   ├── auth/
│       │   ├── catalog/
│       │   ├── collections/
│       │   ├── holdings/
│       │   ├── config/
│       │   ├── errors/
│       │   └── main.rs
│       ├── migrations/
│       └── tests/
├── fixtures/
│   ├── demo-catalog/
│   └── provider-responses/
├── docs/
│   ├── architecture/
│   ├── decisions/
│   ├── api/
│   ├── security/
│   └── technical-specification.md
├── docker-compose.yml
├── Makefile
├── README.md
└── .github/workflows/
```

`packages/ui` e `packages/contracts` só serão criados quando existir reutilização concreta ou geração de contratos. Diretórios vazios não demonstram arquitetura.

## 14. Estratégia de catálogo

### 14.1 Princípio

O uso normal nunca consulta diretamente o provider externo. O frontend lê o catálogo local por meio da API Bycard.

```text
Provider → adaptador → normalização → validação → PostgreSQL → API → frontend
```

### 14.2 Catálogo fictício inicial

- duas coleções;
- dezoito cartas por coleção;
- duas páginas completas de fichário;
- imagens visualmente consistentes;
- alguns campos opcionais ausentes para testar tolerância;
- IDs estáveis e determinísticos.

Exemplo de fixture:

```json
{
  "sets": [
    {
      "id": "astral-origins",
      "name": "Origens Astrais",
      "releaseDate": "2026-01-15",
      "cards": [
        {
          "id": "astral-origins-001",
          "number": "001",
          "name": "Raposa Solar",
          "rarity": "common",
          "image": "/demo/astral-origins/001.webp"
        }
      ]
    }
  ]
}
```

### 14.3 Provider futuro

```rust
pub trait CatalogProvider {
    async fn list_sets(
        &self,
        language: &str,
    ) -> Result<Vec<ExternalSet>, ProviderError>;

    async fn get_set(
        &self,
        external_id: &str,
        language: &str,
    ) -> Result<ExternalSetDetails, ProviderError>;

    async fn list_cards(
        &self,
        set_external_id: &str,
        language: &str,
    ) -> Result<Vec<ExternalCard>, ProviderError>;
}
```

O primeiro release implementará apenas o provider de arquivo/demonstração. Depois, um único provider externo será suficiente para provar substituibilidade.

### 14.4 Requisitos futuros de sincronização

- idempotência;
- timeout;
- retry limitado com backoff;
- respeito a rate limits;
- validação antes da escrita;
- transação por unidade de sincronização;
- preservação do catálogo anterior em caso de falha;
- origem e data da última sincronização;
- contadores de itens criados e atualizados;
- nenhuma exclusão automática em massa;
- fixtures para testes de contrato.

## 15. Modelo de dados do MVP

O modelo inicial não representa variantes, condição ou idioma da posse. Esses conceitos ainda não têm regras de produto suficientes e seriam complexidade prematura.

IDs internos usarão UUID v7 gerado pela aplicação. O scaffold deverá adotar uma única biblioteca Rust com suporte estável e impedir estratégias concorrentes de geração. A decisão evita dependência de extensão específica do PostgreSQL e mantém o ID disponível antes do `INSERT`.

### 15.1 `users`

```text
id
display_name
email
password_hash
created_at
updated_at
deleted_at
```

Restrições:

- e-mail normalizado e único entre contas ativas;
- `display_name` com limites definidos;
- senha nunca armazenada fora do hash.

### 15.2 `sessions`

```text
id
user_id
token_hash
expires_at
created_at
last_seen_at
revoked_at
```

O token opaco deverá ter alta entropia. O banco armazena um hash determinístico apropriado para lookup, e não o token puro. `last_seen_at` não precisa ser atualizado em toda requisição; pode usar uma janela para evitar escrita excessiva.

### 15.3 `games`

```text
id
slug
name
is_active
created_at
```

A tabela mantém o núcleo semanticamente neutro, embora apenas um jogo fictício seja usado no MVP.

### 15.4 `sets`

```text
id
game_id
external_key
slug
name
series_name
release_date
total_cards
cover_image_url
language
is_published
created_at
updated_at
```

Restrições:

```text
UNIQUE(game_id, external_key, language)
UNIQUE(game_id, slug)
CHECK(total_cards >= 0)
```

`total_cards` pode ser exibido como metadado, mas o progresso deve usar a contagem de cartas elegíveis realmente persistidas.

### 15.5 `cards`

```text
id
set_id
external_key
local_number
printed_number
name
rarity
artist
image_small_url
image_large_url
sort_order
metadata_json
is_published
created_at
updated_at
```

Restrições:

```text
UNIQUE(set_id, external_key)
UNIQUE(set_id, sort_order)
CHECK(sort_order >= 0)
```

A fonte e o idioma são derivados da coleção para impedir combinações incoerentes.

### 15.6 `user_collections`

```text
id
user_id
set_id
created_at
updated_at
```

Restrição:

```text
UNIQUE(user_id, set_id)
```

### 15.7 `user_card_holdings`

```text
id
user_id
card_id
quantity
first_obtained_at
updated_at
```

Restrições:

```text
UNIQUE(user_id, card_id)
CHECK(quantity > 0)
```

Ausência de linha significa que a carta não é possuída. Quantidade zero deverá remover a linha, evitando dois estados equivalentes.

### 15.8 Integridade e autorização

- toda operação de posse usa o usuário obtido da sessão, nunca um `user_id` fornecido pelo cliente;
- uma posse só pode ser criada para uma coleção acompanhada pelo usuário;
- remover uma coleção pessoal e suas posses ocorre atomicamente;
- deleções e atualizações devem verificar propriedade no próprio comando SQL ou dentro da mesma transação;
- estatísticas ignoram coleções e cartas não publicadas, conforme regra definida.

## 16. Evolução futura do modelo

Depois do MVP, poderão ser adicionados:

### 16.1 Fontes e sincronizações

```text
catalog_sources
sync_runs
```

### 16.2 Variantes e detalhes de posse

```text
card_variants
holding_groups ou expansão de user_card_holdings
condition
language
notes
```

Antes dessa migration, deverão ser respondidas:

- uma variante conta para o progresso principal?
- cópias em idiomas diferentes são agregadas?
- condição diferencia grupos de posse?
- uma edição traduzida é outra coleção ou o mesmo catálogo?
- como dados antigos migram para a variante padrão?

## 17. Cálculos de coleção

O backend é a fonte de verdade.

```text
total_unique = cartas publicadas e elegíveis da coleção
owned_unique = cartas elegíveis com quantity > 0
missing_unique = total_unique - owned_unique
total_copies = soma de quantity
duplicate_copies = soma de max(quantity - 1, 0)
completion_percentage = owned_unique / total_unique × 100
```

Regras:

- coleção vazia retorna 0%, não divisão por zero;
- quantidades repetidas não aumentam o progresso;
- o valor armazenado em `sets.total_cards` não substitui a contagem real;
- arredondamento é apenas de apresentação;
- o frontend não recalcula regras de negócio;
- o endpoint que altera posse devolve o estado persistido e as estatísticas atualizadas.

## 18. API interna

Prefixo:

```text
/api/v1
```

### 18.1 Convenções

- JSON em requisições e respostas;
- datas em ISO 8601 UTC;
- IDs como strings;
- paginação com limites máximos;
- código de erro estável e mensagem segura;
- `request_id` devolvido ao cliente;
- nenhuma resposta contém hash, segredo ou detalhes internos.

Formato de erro sugerido:

```json
{
  "error": {
    "code": "invalid_credentials",
    "message": "E-mail ou senha inválidos.",
    "requestId": "019..."
  }
}
```

### 18.2 Autenticação

```text
POST /auth/register
POST /auth/login
POST /auth/logout
GET  /auth/me
```

`POST /auth/logout-all` fica para uma evolução posterior, salvo se sua implementação for trivial depois da estrutura de sessões.

### 18.3 Catálogo

```text
GET /sets
GET /sets/:set_id
GET /sets/:set_id/cards
```

Parâmetros possíveis:

```text
search
page
page_size
sort
```

O filtro `ownership` pertence ao contexto autenticado de uma coleção pessoal, não necessariamente ao endpoint público de catálogo.

### 18.4 Coleções pessoais

```text
GET    /me/collections
POST   /me/collections
GET    /me/collections/:set_id
DELETE /me/collections/:set_id
```

Criação:

```json
{
  "setId": "019..."
}
```

### 18.5 Posses

Uma única operação idempotente reduz ambiguidades:

```text
PUT /me/collections/:set_id/cards/:card_id
```

```json
{
  "quantity": 2
}
```

Regras:

- quantidade positiva cria ou substitui;
- quantidade zero remove;
- quantidade negativa é inválida;
- existe um limite superior defensivo;
- `card_id` deve pertencer ao `set_id` da rota;
- o usuário deve acompanhar a coleção.

### 18.6 Saúde

```text
GET /health/live
GET /health/ready
```

Liveness informa que o processo está vivo. Readiness confirma que dependências necessárias, como o banco, permitem atender tráfego.

## 19. Autenticação e segurança

### 19.1 Senhas

- Argon2id com parâmetros configurados e testados no ambiente de produção;
- salt individual gerado pela biblioteca;
- limite mínimo e máximo de comprimento;
- nenhum log de senha ou corpo de autenticação;
- mensagem de credenciais inválidas sem enumeração de e-mails.

### 19.2 Sessões

- token opaco aleatório de alta entropia;
- somente o hash do token no banco;
- cookie `HttpOnly`;
- cookie `Secure` em produção;
- `SameSite=Lax` como base;
- `Path=/` ou escopo mínimo coerente;
- expiração absoluta;
- rotação após autenticação;
- revogação no logout;
- rejeição de sessões expiradas ou revogadas.

### 19.3 CSRF e topologia

Frontend e API devem preferencialmente compartilhar o mesmo site. Toda operação mutável autenticada por cookie deverá ter uma estratégia explícita contra CSRF. `SameSite` é uma camada, não a documentação completa da proteção.

Se forem usados domínios ou sites diferentes, cookies, CORS e credenciais deverão receber testes específicos antes do deploy.

### 19.4 Proteções da API

- validação de entrada e rejeição de campos inesperados quando apropriado;
- limite de corpo;
- timeout por requisição;
- rate limit em cadastro e login;
- CORS restrito;
- cabeçalhos de segurança;
- queries parametrizadas;
- autorização por recurso;
- segredos somente no backend;
- erros internos ocultos;
- redaction de dados sensíveis nos logs.

Rate limiting em memória é aceitável para uma única instância no MVP. Se houver múltiplas instâncias, sua limitação deve ser declarada ou a implementação deve ser compartilhada.

### 19.5 Privacidade

- perfis privados por padrão;
- nenhuma exibição pública de e-mail, localização ou valor de coleção;
- coleta mínima de métricas;
- endereço IP não será persistido no MVP sem finalidade e retenção definidas;
- exclusão de conta completa será implementada antes de aceitar usuários reais fora de uma demonstração controlada.

## 20. Estados e erros da interface

Cada tela deve considerar:

- carregamento inicial;
- atualização em andamento;
- ausência de dados;
- sucesso;
- erro recuperável;
- erro definitivo;
- sessão expirada;
- conexão indisponível;
- catálogo vazio;
- imagem indisponível.

Mensagens sugeridas:

> Você ainda não acompanha nenhuma coleção. Escolha uma para começar seu fichário.

> Não foi possível carregar os dados agora. Tente novamente.

> Sua sessão expirou. Entre novamente para continuar.

Respostas otimistas devem reverter visualmente a alteração se a persistência falhar.

## 21. Testes

### 21.1 Unitários

- progresso;
- repetidas;
- quantidade mínima e máxima;
- transição entre possuída e faltante;
- normalização de catálogo;
- validação de autenticação;
- mapeamento seguro de erros.

### 21.2 Integração

- migrations em banco limpo;
- cadastro e login;
- criação, expiração e revogação de sessões;
- importação idempotente;
- criação de coleção pessoal;
- alteração de quantidade;
- remoção transacional de coleção;
- isolamento entre usuários;
- constraints do banco;
- cálculo de estatísticas a partir de dados reais.

### 21.3 Contrato de providers

- respostas armazenadas em fixtures;
- campos ausentes;
- IDs duplicados;
- timeout e erro do fornecedor;
- nenhuma dependência da API real na suíte padrão.

### 21.4 E2E

Fluxo crítico:

1. criar conta;
2. adicionar coleção;
3. marcar carta;
4. confirmar progresso;
5. recarregar a página;
6. confirmar persistência;
7. remover carta;
8. sair;
9. confirmar proteção da página autenticada.

### 21.5 Segurança

- acesso sem sessão;
- token inválido, expirado e revogado;
- tentativa de acessar recurso de outro usuário;
- enumeração de e-mails;
- payload excessivo;
- campos inesperados;
- busca com entradas hostis;
- configuração de cookies em produção;
- ausência de segredos em respostas e logs;
- CSRF conforme estratégia escolhida.

## 22. Observabilidade

Logs estruturados:

```text
timestamp
level
request_id
route
method
status
duration_ms
user_id quando necessário e permitido
error_code
provider
sync_run_id
```

Nunca registrar:

- senha;
- token de sessão;
- cookie completo;
- chave externa;
- corpo de autenticação;
- dados pessoais sem finalidade.

Métricas iniciais:

- requisições, erros e duração por rota;
- falhas de login;
- disponibilidade do banco;
- importações concluídas e com erro.

Métricas de produto, quando houver usuários reais:

- cadastro concluído;
- primeira coleção adicionada;
- primeira carta registrada;
- tempo até a primeira carta;
- retorno após sete dias.

Eventos de produto devem evitar dados de cartas específicos quando contagens agregadas forem suficientes.

## 23. Configuração

Variáveis previstas:

```text
APP_ENV
API_HOST
API_PORT
DATABASE_URL
SESSION_COOKIE_NAME
SESSION_TTL
CORS_ALLOWED_ORIGINS
CATALOG_PROVIDER
CATALOG_FILE_PATH
LOG_LEVEL
```

Variáveis de providers externos somente quando implementados:

```text
EXTERNAL_CATALOG_BASE_URL
EXTERNAL_CATALOG_API_KEY
```

Regras:

- `.env` ignorado pelo Git;
- `.env.example` sem segredos;
- validação da configuração na inicialização;
- falha rápida quando variável obrigatória estiver ausente;
- frontend nunca recebe credenciais de provider;
- uma variável `SESSION_SECRET` só será criada se houver uma finalidade criptográfica documentada.

## 24. Desenvolvimento local

Experiência desejada:

```bash
git clone <repository>
cd Bycard
cp .env.example .env
docker compose up -d db
pnpm install
cargo sqlx migrate run
make dev
```

Comandos padronizados previstos:

```bash
make setup
make dev
make test
make lint
make format
make migrate
make seed
make import-demo
```

O critério não é necessariamente executar todos os processos em um único comando, mas oferecer um procedimento previsível, documentado e reproduzível.

## 25. CI/CD

### 25.1 Frontend

- instalação com lockfile;
- lint;
- typecheck;
- testes;
- build.

### 25.2 Backend

- `cargo fmt --check`;
- Clippy com política de warnings definida;
- testes unitários e de integração;
- build release;
- auditoria de dependências.

### 25.3 SQLx

A estratégia deverá ser escolhida explicitamente:

- banco PostgreSQL disponível na CI; ou
- metadados offline do SQLx atualizados e versionados.

A pipeline não deve depender acidentalmente de um banco local inexistente.

### 25.4 Entrega

- build de imagens reprodutíveis;
- migrações controladas;
- health check após deploy;
- bloqueio de promoção em caso de falha;
- rollback inicialmente manual e documentado, antes de automatização.

## 26. Deploy

Ambiente de demonstração:

- frontend;
- API;
- PostgreSQL;
- catálogo fictício;
- HTTPS;
- dados previsíveis;
- conta de demonstração opcional sem credenciais privilegiadas.

Requisitos:

- frontend e API preferencialmente no mesmo site;
- backup compatível com a importância dos dados;
- variáveis protegidas;
- logs e health checks;
- migrações versionadas;
- domínio próprio do Bycard sem sugerir caráter oficial;
- páginas de erro;
- política de privacidade antes de coleta real de dados pessoais.

## 27. Roadmap de execução

### Fase 0 — decisões mínimas

- consolidar esta especificação;
- definir regras de posse e progresso;
- desenhar quatro wireframes mobile-first;
- criar catálogo fictício;
- registrar somente ADRs realmente decididos.

Conclusão:

> O fluxo principal e o modelo mínimo não possuem ambiguidades que impeçam migrations.

### Fase 1 — fundação

- iniciar Git;
- configurar frontend e backend;
- configurar PostgreSQL;
- criar Compose e variáveis;
- criar migrations iniciais;
- implementar liveness e readiness;
- configurar lint, formatação e CI básica.

Conclusão:

> Frontend, API e banco executam por um procedimento documentado.

### Fase 2 — fatia vertical do catálogo

- importar fixture idempotentemente;
- expor coleções e cartas;
- criar catálogo visual;
- implementar fichário somente leitura;
- busca e estados de erro;
- publicar versão preliminar.

Conclusão:

> O catálogo fictício navega de ponta a ponta em celular e computador.

### Fase 3 — autenticação

- cadastro;
- hash de senha;
- login;
- sessão e cookie;
- logout;
- `/auth/me`;
- middleware;
- rate limit básico;
- testes de segurança essenciais.

Conclusão:

> O usuário entra, recarrega a página e permanece autenticado; sessões inválidas são rejeitadas.

### Fase 4 — coleções e posses

- adicionar e remover coleção;
- alterar quantidade;
- filtrar por posse;
- calcular progresso;
- atualizar interface;
- testar isolamento entre usuários.

Conclusão:

> Dois usuários possuem dados isolados e o progresso é consistente com o banco.

### Fase 5 — qualidade visual e técnica

- acessibilidade;
- responsividade;
- otimização de imagens;
- E2E;
- revisão de segurança;
- logs;
- documentação;
- screenshots e vídeo.

Conclusão:

> Nenhuma falha conhecida impede o fluxo crítico e a apresentação pública.

### Fase 6 — publicação

- banco e domínio de produção;
- HTTPS;
- migrações;
- seed de demonstração;
- monitoramento básico;
- README final;
- licença e limitações.

Conclusão:

> Um avaliador consegue acessar, testar e entender o projeto sem ajuda do autor.

## 28. Evoluções pós-MVP

### 28.1 Versão 1.1

- ordenação e filtros combinados;
- notas;
- exportação JSON ou CSV;
- instalação como PWA;
- exclusão de conta completa;
- recuperação e verificação de e-mail.

### 28.2 Versão 1.2

- variantes reais;
- idiomas;
- condições;
- histórico de alterações;
- modo de abertura de pacote.

### 28.3 Versão 1.3

- wishlist;
- favoritas;
- disponíveis para troca;
- perfil compartilhável opt-in;
- controles adicionais de privacidade.

### 28.4 Versão 2.0

- segundo jogo;
- reconhecimento por imagem;
- preços;
- comparação entre usuários;
- sugestões de troca;
- notificações.

Cada versão começa somente depois de a anterior estar publicada, utilizável e estável.

## 29. Modo futuro de abertura de pacotes

Fluxo previsto:

1. escolher coleção;
2. iniciar sessão;
3. informar quantidade de pacotes;
4. registrar cartas;
5. finalizar sessão;
6. calcular novas e repetidas;
7. atualizar progresso;
8. salvar histórico.

Entidades candidatas:

```text
pack_openings
pack_opening_entries
```

Exemplo de resumo:

```text
10 cartas adicionadas
6 novas
4 repetidas
Progresso anterior: 21%
Progresso atual: 24%
```

## 30. Critérios de sucesso

### 30.1 Marco 1

- importação repetida não duplica registros;
- catálogo é servido do PostgreSQL;
- duas coleções são navegáveis;
- busca funciona;
- fichário funciona em celular e desktop;
- nenhuma API externa é necessária.

### 30.2 MVP

- cadastro, login, persistência e logout funcionam;
- usuário acompanha ao menos uma coleção;
- quantidades são preservadas;
- progresso e faltantes estão corretos;
- dados de usuários permanecem isolados;
- catálogo fictício funciona em produção;
- fluxos críticos possuem testes;
- CI está verde;
- não há segredos versionados;
- README explica instalação, arquitetura, decisões e limitações;
- não há funcionalidade principal marcada como “em breve”.

## 31. Riscos e respostas

### 31.1 Escopo excessivo

**Risco:** o projeto acumular infraestrutura e não ser publicado.  
**Resposta:** trabalhar por fatias verticais; nenhuma função pós-MVP bloqueia a entrega.

### 31.2 Propriedade intelectual

**Risco:** uso indevido de marcas e imagens.  
**Resposta:** marca própria, aviso de guia não oficial, nenhuma arte oficial incorporada ao repositório, provider desligável e verificação individual de licenças e termos.

### 31.3 Dependência externa

**Risco:** provider ficar indisponível ou mudar.  
**Resposta:** catálogo local, adaptador, fixtures e preservação dos dados existentes.

### 31.4 Modelo prematuramente genérico

**Risco:** variantes, idiomas e condições multiplicarem estados sem regras claras.  
**Resposta:** holdings mínimos no MVP e migrations posteriores baseadas em requisitos reais.

### 31.5 Progresso incorreto

**Risco:** metadados de total divergirem das cartas persistidas.  
**Resposta:** denominador derivado de cartas elegíveis do banco e testes de integração.

### 31.6 Interface pesada

**Risco:** imagens e animações prejudicarem celular.  
**Resposta:** thumbnails, dimensões reservadas, lazy loading, paginação e movimento reduzido.

### 31.7 Comprometimento de contas

**Risco:** credenciais ou sessões expostas.  
**Resposta:** Argon2id, tokens opacos, cookies seguros, CSRF, rate limiting, autorização por recurso e logs sem segredos.

### 31.8 Complexidade do frontend e backend separados

**Risco:** cookies, CORS e deploy ficarem mais frágeis.  
**Resposta:** builds independentes, mas exposição sob o mesmo site e contrato HTTP simples.

## 32. Decisões arquiteturais a registrar

ADRs iniciais aceitos:

```text
ADR-001 — Monólito modular
ADR-002 — PostgreSQL e SQLx
ADR-003 — Sessões opacas em vez de JWT no navegador
ADR-004 — Catálogo local importado de fixture
ADR-005 — Holdings mínimos no MVP
ADR-006 — Frontend e API sob o mesmo site
```

Cada ADR deve registrar contexto, opções consideradas, decisão, consequências e condições para revisão. Não deve apenas repetir a tecnologia escolhida.

## 33. Documentação de portfólio

O README final deverá conter:

- problema e solução;
- demonstração;
- screenshots e vídeo curto;
- tecnologias e versões;
- arquitetura;
- instalação;
- configuração;
- testes;
- decisões;
- segurança;
- limitações;
- roadmap;
- aviso de independência;
- licença do código.

Diagramas úteis:

- arquitetura geral;
- fluxo de sessão;
- importação/sincronização;
- modelo relacional;
- registro de posse.

Aspectos técnicos a destacar:

- importação idempotente;
- fronteira substituível de provider;
- segurança da sessão;
- isolamento entre usuários;
- integridade das posses;
- cálculo de progresso;
- fichário responsivo;
- testes de integração.

## 34. Descrição pública

### 34.1 GitHub

> Bycard é uma plataforma de gerenciamento de coleções de cartas construída com Next.js, Rust, Axum e PostgreSQL. O sistema utiliza um catálogo local importado por uma fronteira substituível, permite registrar cartas e quantidades, identifica itens faltantes e apresenta o progresso em um fichário digital responsivo.

### 34.2 Portfólio

> Desenvolvi o Bycard para resolver um problema que surgiu ao acompanhar coleções de cartas: saber quais itens já possuo e quais ainda faltam. A aplicação importa e normaliza catálogos em um banco local, registra posses sem depender da disponibilidade do fornecedor durante o uso e apresenta o progresso em um fichário responsivo. O projeto utiliza Next.js, TypeScript, Rust, Axum, SQLx e PostgreSQL, com foco em segurança de sessões, integridade relacional, testes e experiência mobile-first.

## 35. Definição final de pronto

Bycard estará pronto para apresentação quando:

- possuir demonstração online;
- utilizar marca própria, aviso de guia não oficial e catálogo funcional sem provider externo;
- funcionar sem API externa;
- autenticação e autorização estiverem verificadas;
- coleção pessoal estiver funcional;
- progresso estiver correto;
- fichário funcionar em celular e computador;
- testes críticos estiverem aprovados;
- CI estiver verde;
- documentação estiver completa;
- limitações estiverem declaradas;
- nenhum segredo estiver no repositório;
- nenhum item essencial aparecer apenas como promessa futura.

## 36. Próxima ação concreta

A próxima etapa não é criar todas as tabelas nem todos os módulos. É executar a fundação do Marco 1 nesta ordem:

1. inicializar o repositório Git;
2. definir os quatro wireframes;
3. produzir o catálogo fictício mínimo;
4. criar o Compose com PostgreSQL;
5. iniciar Axum e implementar health checks;
6. iniciar Next.js;
7. criar migrations de `games`, `sets` e `cards`;
8. importar a fixture idempotentemente;
9. expor endpoints de leitura;
10. construir o fichário somente leitura;
11. testar;
12. publicar a primeira fatia.

Qualquer nova funcionalidade deve responder afirmativamente a uma pergunta: ela é necessária para concluir o marco atual? Se não for, entra no backlog, não na implementação corrente.
