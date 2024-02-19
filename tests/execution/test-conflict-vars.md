# test-conflict-vars

#### server:

```graphql
schema @server(vars: [{key: "a", value: "b"}, {key: "c", value: "d"}]) @upstream {
  query: Query
}

type Query {
  hello: String @const(data: "world")
}
```

#### server:

```graphql
schema @server(vars: [{key: "a", value: "b"}, {key: "p", value: "q"}]) @upstream {
  query: Query
}

type Query {
  hello: String @const(data: "world")
}
```
