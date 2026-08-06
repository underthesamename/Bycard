# API de catálogo

O catálogo público do Bycard é servido exclusivamente a partir do PostgreSQL local. Os endpoints não exigem autenticação e nunca retornam coleções ou cartas com `is_published = false`.

Base: `/api/v1`

## Paginação e filtros

As listagens retornam `data` e `pagination`:

```json
{
  "data": [],
  "pagination": {
    "page": 1,
    "pageSize": 20,
    "totalItems": 0,
    "totalPages": 0
  }
}
```

- `page`: padrão `1`, mínimo `1`;
- `pageSize`: padrão `20`, mínimo `1`, máximo `100`;
- `search`: busca sem diferença entre maiúsculas e minúsculas; espaços consecutivos são normalizados;
- `sort`: aceita somente os valores documentados em cada endpoint;
- uma listagem vazia possui `totalPages: 0`.

Caracteres especiais de SQL e curingas de `LIKE` recebidos em `search` são tratados como texto. Toda consulta usa parâmetros do PostgreSQL.

## `GET /sets`

Lista coleções publicadas. A busca considera o nome.

Ordenações: `release_date_desc` (padrão), `release_date_asc`, `name_asc` e `name_desc`.

```http
GET /api/v1/sets?search=Horizonte&page=1&pageSize=20&sort=release_date_desc
```

```json
{
  "data": [
    {
      "id": "0198...",
      "slug": "horizonte-solar",
      "name": "Horizonte Solar",
      "seriesName": "Atlas de Luz",
      "releaseDate": "2026-01-17",
      "totalCards": 18,
      "coverImageUrl": "/demo/placeholders/horizonte-solar.svg",
      "language": "pt-BR"
    }
  ],
  "pagination": {
    "page": 1,
    "pageSize": 20,
    "totalItems": 1,
    "totalPages": 1
  }
}
```

## `GET /sets/:set_id`

Retorna uma coleção publicada por UUID.

```json
{
  "data": {
    "id": "0198...",
    "slug": "horizonte-solar",
    "name": "Horizonte Solar",
    "seriesName": "Atlas de Luz",
    "releaseDate": "2026-01-17",
    "totalCards": 18,
    "coverImageUrl": null,
    "language": "pt-BR"
  }
}
```

UUID malformado retorna `400`; UUID válido sem coleção publicada retorna `404`.

## `GET /sets/:set_id/cards`

Lista cartas publicadas de uma coleção publicada. A busca considera nome, número local e número impresso.

Ordenações: `number_asc` (padrão), `number_desc`, `name_asc` e `name_desc`.

```http
GET /api/v1/sets/0198.../cards?search=016&page=1&pageSize=20&sort=number_asc
```

```json
{
  "data": [
    {
      "id": "0198...",
      "setId": "0198...",
      "localNumber": "016",
      "printedNumber": "016/018",
      "name": "Silêncio Azul",
      "rarity": "rara",
      "artist": "Ateliê Bycard",
      "imageSmallUrl": null,
      "imageLargeUrl": null,
      "sortOrder": 16
    }
  ],
  "pagination": {
    "page": 1,
    "pageSize": 20,
    "totalItems": 1,
    "totalPages": 1
  }
}
```

## Erros e rastreamento

Toda resposta inclui o header `x-request-id`. Erros da API repetem o mesmo identificador no envelope, sem expor SQL ou detalhes internos:

```json
{
  "error": {
    "code": "invalid_parameter",
    "message": "pageSize deve ser no máximo 100.",
    "requestId": "0198..."
  }
}
```

Códigos atuais:

- `invalid_id`: UUID da coleção malformado;
- `invalid_query`: query string impossível de interpretar;
- `invalid_parameter`: paginação, busca ou ordenação inválida;
- `catalog_not_found`: coleção publicada inexistente;
- `internal_error`: falha interna opaca e correlacionada pelo `requestId`.
