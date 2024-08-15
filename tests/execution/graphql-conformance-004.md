---
skip: true
---

# Test complex aliasing.

TODO: Skipped because Tailcall does not send the alias to the remote server.

```graphql @config
schema
  @server(port: 8001, queryValidation: false, hostname: "0.0.0.0")
  @upstream(baseURL: "http://upstream/graphql", httpCache: 42) {
  query: Query
}

type Query {
  user(id: ID!): User! @graphQL(name: "user", args: [{key: "id", value: "{{.args.id}}"}])
}

type User {
  id: ID!
  name: String!
  profilePic(size: Int, width: Int, height: Int): String!
}
```

```yml @mock
- request:
    method: POST
    url: http://upstream/graphql
    textBody: '{ "query": "query { user(id: 4) { id name smallPic: profilePic(size: 64) bigPic: profilePic(size: 1024) } }" }'
  expectedHits: 1
  response:
    status: 200
    body:
      data:
        user:
          id: 4
          name: Tailcall
          profilePic: invalid
          smallPic: pic_100
          bigPic: pic_1024
```

```yml @test
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      {
        user(id: 4) {
          id
          name
          smallPic: profilePic(size: 64)
          bigPic: profilePic(size: 1024)
        }
      }
```
