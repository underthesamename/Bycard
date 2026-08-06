# Decisões abertas

**Última revisão:** 5 de agosto de 2026

## Bloqueadores atuais

Não há decisões abertas que impeçam o scaffold ou as migrations iniciais do catálogo.

## Substituição visual não bloqueante

Os oito SVGs em `apps/web/public/demo/placeholders` são ilustrações próprias provisórias para o catálogo inicial. Eles preservam o fluxo offline e não imitam cartas oficiais. Antes da publicação final, deverão ser avaliados como conteúdo visual e poderão ser substituídos por ilustrações autorais mais detalhadas sem alterar IDs ou contratos do catálogo.

## Decisões encerradas nesta revisão

- IDs internos usarão UUID v7 gerado pela aplicação. O scaffold deverá escolher uma biblioteca Rust com suporte estável, desabilitar estratégias concorrentes de geração e testar unicidade e versão. Isso evita extensão específica do PostgreSQL e permite conhecer o ID antes do `INSERT`.
- Ausência de linha em `user_card_holdings` equivale a quantidade zero. Quantidade positiva persiste a linha; quantidade zero a remove. Registrado em `ADR-005`.
- Remover uma coleção pessoal exclui suas holdings na mesma transação, após confirmação explícita. Registrado em `ADR-005`.
- O progresso usa a contagem de cartas publicadas e elegíveis persistidas. Metadados de total do catálogo não são a fonte de verdade.

## Política deste arquivo

Somente decisões não resolvidas que bloqueiem o próximo trabalho devem permanecer em “Bloqueadores atuais”. Questões futuras sobre variantes, providers externos, PWA ou modo de abertura de pacotes pertencem ao backlog até que o MVP publicado forneça evidência para decidi-las.
