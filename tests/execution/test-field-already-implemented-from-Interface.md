# test-field-already-implemented-from-Interface

###### sdl error

#### server:

```graphql
schema {
  query: Query
}

interface IUser {
  id: ID
  name: String
}
type User implements IUser {
  userName: String! @modify(name: "name")
  userId: ID! @modify(name: "id")
}

type Query {
  user: User @http(path: "/user/{{args.input.id}}", baseURL: "http://localhost:8080")
}
```
