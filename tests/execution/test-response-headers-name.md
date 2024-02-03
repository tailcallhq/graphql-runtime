# test-response-headers-name

###### sdl error

#### server:

```graphql
schema @server(responseHeaders: [{key: "🤣", value: "a"}]) {
  query: Query
}

type User {
  name: String
  age: Int
}

type Query {
  user: User @const(data: {name: "John"})
}
```
