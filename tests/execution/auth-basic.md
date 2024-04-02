# Auth with BasicAuth

```graphql @server
schema @server(port: 8000, graphiql: true) @link(id: "htpasswd", type: Htpasswd, src: ".htpasswd") {
  query: Query
}

type Query {
  scalar: String! @const(data: "data from public scalar")
  protectedScalar: String! @protected @const(data: "data from protected scalar")
  nested: Nested! @const(data: {name: "nested name", protected: "protected nested"})
  protectedType: ProtectedType
}

type Nested {
  name: String!
  protected: String! @protected
}

type ProtectedType @protected {
  name: String! @const(data: "protected type name")
  nested: String! @const(data: "protected type nested")
}
```

```text @file:.htpasswd
testuser1:$apr1$e3dp9qh2$fFIfHU9bilvVZBl8TxKzL/
testuser2:$2y$10$wJ/mZDURcAOBIrswCAKFsO0Nk7BpHmWl/XuhF7lNm3gBAFH3ofsuu
testuser3:{SHA}Y2fEjdGT1W6nsLqtJbGUVeUp9e4=
```

```yml @assert
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      query {
        scalar
        nested {
          name
        }
      }
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      query {
        protectedScalar
      }
- method: POST
  url: http://localhost:8080/graphql
  headers:
    Authorization: Basic dGVzdHVzZXIxOnJhbmRvbV9wYXNzd29yZA==
  body:
    query: |
      query {
        protectedScalar
      }
- method: POST
  url: http://localhost:8080/graphql
  headers:
    Authorization: Basic dGVzdHVzZXIxOnBhc3N3b3JkMTIz
  body:
    query: |
      query {
        protectedScalar
        nested {
          name
          protected
        }
        protectedType {
          name
          nested
        }
      }
```
