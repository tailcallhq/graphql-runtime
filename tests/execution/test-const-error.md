---
expect_validation_error: true
---

# test-const-error

```graphql @server
schema @server @upstream(baseURL: "https://jsonplaceholder.typicode.com") {
  query: Query
}

type User {
  name: String
  age: Int!
}

type Query {
  user: User @const(data: {name: "John"})
}
```
