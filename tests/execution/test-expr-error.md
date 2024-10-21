---
error: true
---

# test-expr-error

```graphql @config
schema @server {
  query: Query
}

type User {
  name: String
  age: Int!
}

type Query {
  user: User @expr(body: {name: "John"})
}
```
