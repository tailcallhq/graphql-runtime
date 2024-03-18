---
expect_validation_error: true
---

# test-expr-errors

```graphql @server
schema @server {
  query: Query
}

type Query {
  foo: String @expr(data: {const: "John"})
  bar: String @expr(body: {unsupported: true})
}
```
