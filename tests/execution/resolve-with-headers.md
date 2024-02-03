# Resolve with headers

#### server:

```graphql
schema @upstream(allowedHeaders: ["authorization"]) {
  query: Query
}

type Post {
  id: ID!
  title: String!
  body: String!
  userId: ID!
}

type Query {
  post1: Post @http(path: "/posts/{{headers.authorization}}", baseURL: "http://jsonplaceholder.typicode.com")
}
```

#### mock:

```yml
- request:
    method: GET
    url: http://jsonplaceholder.typicode.com/posts/1
    body: null
  response:
    status: 200
    headers:
      authorization: "1"
    body:
      id: 1
      title: post title
```

#### assert:

```yml
- method: POST
  url: http://localhost:8080/graphql
  headers:
    authorization: "1"
  body:
    query: query { post1 { title } }
```
