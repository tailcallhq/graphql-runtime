---
identity: true
---

# test-modify

```graphql @config
schema @server @upstream {
  query: Query
}

input Foo {
  bar: String
}

type Query {
  foo(input: Foo): String @http(url: "http://jsonplaceholder.typicode.com/foo") @modify(name: "data")
}
```
