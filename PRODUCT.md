# Bycard

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

- Frontend: Next.js com App Router, TypeScript e Tailwind CSS.
- Backend: Rust com Axum, Tokio, Serde, SQLx, Tower e tracing.
- Persistência: PostgreSQL.
- Desenvolvimento local: Docker Compose.
- Produção: frontend e API com builds independentes, expostos sob o mesmo site.

As versões serão verificadas e fixadas no scaffold. Nenhuma versão de dependência é presumida por este documento.

## Users

O usuário principal é um colecionador iniciante que compra pacotes ou produtos lacrados, organiza cartas em fichários físicos e usa principalmente o celular. Ele quer registrar cartas rapidamente porque não sabe com segurança quais já possui e quais ainda faltam.

Colecionadores experientes, pessoas com várias coleções e usuários com cópias repetidas são públicos secundários. As necessidades deles não ampliam o escopo inicial.

## Product Purpose

Bycard permite escolher uma coleção de Pokémon TCG, registrar cartas e quantidades e acompanhar visualmente posses, faltantes e progresso.

O primeiro sucesso do produto é uma demonstração pública estável e compreensível que prove qualidade de execução técnica. Validar preferência sobre planilhas é uma hipótese posterior: exige catálogo real, usuários reais e evidência de comportamento que ainda não existem.

## Positioning

Bycard é uma implementação própria de gerenciamento de coleções que combina um catálogo local, independente da disponibilidade de fornecedores durante o uso, com uma experiência responsiva inspirada na organização de um fichário físico.

O projeto não reivindica ter criado a categoria nem competir inicialmente com aplicativos estabelecidos. Seu diferencial de portfólio é a qualidade da modelagem, da importação, da segurança, dos testes e da experiência operacional.

## Operating Context

- O registro acontece principalmente pelo celular, inclusive durante a abertura e organização de cartas físicas.
- O usuário precisa localizar uma carta por nome ou número e registrá-la em poucos toques.
- O fichário deve continuar navegável sem consultar um provider externo.
- O catálogo público inicial contém duas coleções fictícias com dezoito cartas cada.
- O fluxo principal é entrar, escolher uma coleção, marcar cartas, observar o progresso e retornar com os dados preservados.

## Capabilities and Constraints

### Primeira fatia

- importar um catálogo fictício idempotentemente para PostgreSQL;
- listar coleções e cartas por uma API própria;
- buscar cartas por nome ou número;
- navegar por um fichário responsivo somente leitura;
- tratar carregamento, vazio, erro e imagem ausente.

### MVP de portfólio

- cadastro, login, sessão persistente e logout;
- coleções pessoais privadas;
- controle de quantidade por carta;
- filtros de cartas possuídas e faltantes;
- progresso por cartas únicas persistidas;
- isolamento entre usuários;
- testes críticos e demonstração publicada.

### Restrições duráveis

- A interface identifica explicitamente o caso de uso Pokémon TCG e deve se apresentar como guia não oficial e não comercial.
- Artes oficiais não são incorporadas ao repositório. Quando habilitadas, imagens reais são fornecidas por um provider configurado e permanecem sujeitas aos termos e direitos da fonte.
- A demonstração sem provider usa placeholders identificados, preservando toda a funcionalidade.
- A demonstração funciona sem API externa.
- Regras e estatísticas pertencem ao backend.
- Perfis são privados por padrão.
- O MVP possui uma posse simples por usuário e carta; variantes, condição e idioma da posse são posteriores.
- Next.js e Axum permanecem aplicações separadas, mas compartilham o mesmo site em produção para simplificar cookies, CORS e CSRF.
- Funcionalidades futuras não devem aumentar a complexidade da primeira entrega.

## Brand Commitments

- Nome do produto: Bycard.
- Idioma inicial da experiência: português brasileiro.
- Marca própria e independente, direcionada a colecionadores de Pokémon TCG.
- Comunicação direta, compreensível para pessoas com baixo conhecimento técnico.
- Identificação visível como “Guia não oficial”.
- Nenhuma alegação de associação, autorização ou endosso por Nintendo, Creatures, Game Freak ou The Pokémon Company.
- Projeto sem fins lucrativos.

A direção visual aprovada está registrada em `docs/design/visual-direction.md`. O sistema visual definitivo será documentado em `DESIGN.md` somente depois de implementado e verificado.

## Evidence on Hand

- Especificação técnica e de produto: `docs/technical-specification.md`.
- Plano sequencial de construção: `docs/construction-prompts.md`.
- Conceito visual aprovado: `docs/design/concepts/bycard-pokemon-theme.webp`.

Não existem ainda protótipo, catálogo fictício produzido, aplicação executável, usuários, métricas, depoimentos, benchmarks ou validação comercial. Trabalhos futuros não devem inventar essas evidências.

## Product Principles

1. Registrar uma carta deve exigir poucos toques.
2. A posse é privada por padrão.
3. O uso normal não depende da disponibilidade de um fornecedor externo.
4. O backend é a fonte de verdade para regras e estatísticas.
5. A engenharia deve ser proporcional ao marco atual.

## Accessibility & Inclusion

- abordagem mobile-first;
- navegação completa por teclado;
- foco visível;
- contraste adequado;
- suporte à preferência por movimento reduzido;
- alvos de toque adequados;
- nenhuma informação transmitida somente por cor;
- mensagens de erro associadas aos controles relevantes;
- atualização de quantidade anunciada por tecnologia assistiva quando necessário.
