# Caching Parent Null

#### server:

```graphql
schema @upstream(baseURL: "http://example.com", batch: {delay: 1, maxSize: 1000}) {
  query: Query
}

type Query @cache(maxAge: 3000) {
  bars: [Bar] @http(path: "/bars")
}

type Bar {
  id: Int!
}
```

#### mock:

```yml
- request:
    method: GET
    url: http://example.com/bars
    body: null
  response:
    status: 200
    body:
      - id: 1
      - id: 3
      - id: 5
      - id: 7
```

#### assert:

```yml
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: query { bars { id } }
```
