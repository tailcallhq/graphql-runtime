# test-batching-group-by

###### check identity

#### server:

```graphql
schema @server(port: 4000) @upstream(baseURL: "http://abc.com", batch: {delay: 1, headers: [], maxSize: 1000}) {
  query: Query
}

type Post {
  body: String
  id: Int
  title: String
  user: User @http(groupBy: ["id"], path: "/users", query: [{key: "id", value: "{{value.userId}}"}])
  userId: Int!
}

type Query {
  posts: [Post] @http(path: "/posts?id=1&id=11")
}

type User {
  id: Int
  name: String
}
```
