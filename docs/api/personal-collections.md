# Coleções pessoais e holdings

Todas as rotas usam o usuário da sessão opaca. Requisições mutáveis também exigem `Origin` válido e o token obtido em `GET /api/v1/auth/csrf` no header `X-CSRF-Token`.

## Contratos

- `GET /api/v1/me/collections` lista somente as coleções da sessão atual.
- `POST /api/v1/me/collections` recebe `{ "setId": "uuid" }`. A primeira inclusão retorna `201`; repetir a mesma coleção é idempotente e retorna `200` com o estado atual.
- `GET /api/v1/me/collections/:set_id` retorna coleção, estatísticas e cartas com quantidade.
- `DELETE /api/v1/me/collections/:set_id` remove a coleção e suas holdings na mesma transação.
- `PUT /api/v1/me/collections/:set_id/cards/:card_id` recebe `{ "quantity": 0..999 }`. Zero remove a holding; valores positivos criam ou substituem.

O `PUT` usa quantidade absoluta e é idempotente. A API bloqueia a coleção pessoal durante cada escrita; a interface desabilita os controles da carta enquanto a mutação está em andamento. Isso evita cliques concorrentes com base visual obsoleta sem introduzir versionamento no MVP. Clientes alternativos devem aguardar a resposta antes de calcular e enviar a próxima quantidade.

As estatísticas são calculadas no backend a partir das cartas publicadas persistidas. `sets.total_cards` não é usado como denominador.
