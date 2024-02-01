# With args

#### server:

```graphql
schema {
  query: Query
}

type User {
  name: String
}

type Query {
  user(id: Int!): [User]
    @http(path: "/users", query: [{key: "id", value: "{{args.id}}"}], baseURL: "http://jsonplaceholder.typicode.com")
}
```

#### mock:

```yml
- request:
    method: GET
    url: http://jsonplaceholder.typicode.com/users?id=1
    body: null
  response:
    status: 200
    body:
      - id: 1
        name: Leanne Graham
```

#### assert:

```yml
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: "query { user(id: 1) { name } }"
```
