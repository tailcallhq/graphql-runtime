---
check_identity: true
---

# test-enum

```graphql @server
schema @server @upstream(baseURL: "http://localhost:8080") {
  query: Query
}

enum Foo {
  BAR
  BAZ
}

type Query {
  foo(val: String!): Foo @const(data: "{{args.val}}")
}
```

```yml @assert
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: 'query { foo(val: "BAR") }'

- method: POST
  url: http://localhost:8080/graphql
  body:
    query: 'query { foo(val: "BAZ") }'

- method: POST
  url: http://localhost:8080/graphql
  body:
    query: 'query { foo(val: "INVALID") }'
```
