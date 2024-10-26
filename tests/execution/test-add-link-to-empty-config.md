---
identity: true
---

# test-add-link-to-empty-config

```graphql @file:link-expr.graphql
schema @server @upstream {
  query: Query
}

type Query {
  hello: String @expr(body: "Hello from server")
}
```

```graphql @file:link-enum.graphql
schema @server {
  query: Query
}

enum Foo {
  BAR
  BAZ
}

type Query {
  foo: Foo @http(url: "http://jsonplaceholder.typicode.com/foo")
}
```

```graphql @config
schema @server @upstream @link(src: "link-expr.graphql", type: Config) @link(src: "link-enum.graphql", type: Config) {
  query: Query
}
```
