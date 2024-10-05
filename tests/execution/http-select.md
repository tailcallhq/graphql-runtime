# Basic queries with field ordering check

```graphql @config
schema
  @server(port: 8001, queryValidation: false, hostname: "0.0.0.0")
  @upstream(baseURL: "http://upstream", httpCache: 42) {
  query: Query
}

type Query {
  userCompany(id: Int!): Company @http(path: "/users/{{.args.id}}", select: "{{.company}}")
  userDetails(id: Int!): UserDetails
    @http(path: "/users/{{.args.id}}", select: {id: "{{.id}}", city: "{{.address.city}}", phone: "{{.phone}}"})
}

type UserDetails {
  id: Int!
  city: String!
  phone: String!
}

type Company {
  name: String!
  catchPhrase: String!
}
```

```yml @mock
- request:
    method: GET
    url: http://upstream/users/1
  expectedHits: 2
  response:
    status: 200
    body:
      id: 1
      company:
        name: FOO
        catchPhrase: BAR
      address:
        city: FIZZ
      phone: BUZZ
```

```yml @test
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      {
        userCompany(id: 1) {
          name
          catchPhrase
        }
        userDetails(id: 1) {
          city
        }
      }
```
