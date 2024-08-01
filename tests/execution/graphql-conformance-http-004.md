# Test complex aliasing

```graphql @config
schema
  @server(port: 8001, queryValidation: false, hostname: "0.0.0.0")
  @upstream(baseURL: "http://upstream/", httpCache: 42) {
  query: Query
}

type Query {
  user(id: ID!): User! @http(path: "/user", query: [{key: "id", value: "{{.args.id}}"}])
}

type User {
  id: ID!
  name: String!
  profilePic(size: Int, width: Int, height: Int): String!
    @http(
      path: "/pic"
      query: [
        {key: "id", value: "{{.value.id}}"}
        {key: "size", value: "{{.args.size}}"}
        {key: "width", value: "{{.args.width}}"}
        {key: "height", value: "{{.args.height}}"}
      ]
    )
}
```

```yml @mock
- request:
    method: GET
    url: http://upstream/user?id=4
  expectedHits: 1
  response:
    status: 200
    body:
      id: 4
      name: Tailcall
- request:
    method: GET
    url: http://upstream/pic?id=4&size=64&width&height
  expectedHits: 1
  response:
    status: 200
    body: profile_pic_64
- request:
    method: GET
    url: http://upstream/pic?id=4&size=1024&width&height
  expectedHits: 1
  response:
    status: 200
    body: profile_pic_1024
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
