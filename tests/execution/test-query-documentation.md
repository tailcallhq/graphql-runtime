# test-query-documentation

###### check identity

#### server:

```graphql
schema @server @upstream(baseURL: "http://jsonplacheholder.typicode.com") {
  query: Query
}

type Query {
  """
  This is test
  """
  foo: String @http(path: "/foo")
}
```
