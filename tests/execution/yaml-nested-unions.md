# Using union types inside other union types

```graphql @config
schema {
  query: Query
}

type T1 {
  t1: String
}

type T2 {
  t2: Int
}

type T3 {
  t3: Boolean
  t33: Float!
}

type T4 {
  t4: String
}

type T5 {
  t5: Boolean
}

union U1 = T1 | T2 | T3
union U2 = T3 | T4
union U = U1 | U2 | T5

type Query {
  test(u: U!): U @http(url: "http://localhost/users/{{args.u}}")
}
```
