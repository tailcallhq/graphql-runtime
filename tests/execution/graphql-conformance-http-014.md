# Test double query

```graphql @config
schema @server(port: 8001, queryValidation: false, hostname: "0.0.0.0") @upstream(httpCache: 42) {
  query: Query
}

type Query {
  user(id: ID!): User! @http(url: "http://upstream/user", query: [{key: "id", value: "{{.args.id}}"}])
}

type User {
  id: ID!
  name: String!
  city: String
}
```

```yml @mock
- request:
    method: GET
    url: http://upstream/user?id=1
  expectedHits: 1
  response:
    status: 200
    body:
      id: 1
      name: Admin
```

```yml @test
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      query getUser {
        user(id: 1) {
          id
          name
        }
      }

      query getUser {
        user(id: 1) {
          id
          name
        }
      }
- method: POST
  url: http://localhost:8080/graphql
  body:
    operationName: getAdmin
    query: |
      query getAdmin {
        user(id: 1) {
          id
          name
        }
      }

      query getUser {
        user(id: 5) {
          id
          name
        }
      }
```
