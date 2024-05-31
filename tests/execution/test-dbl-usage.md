---
error: true
---

# test-dbl-usage

```graphql @config
schema {
  query: Query
}

type User {
  id: ID!
  name: String!
}

type Query {
  user(input: User!): User @http(path: "/user/{{.args.input.id}}", baseURL: "http://localhost:8080")
}
```
