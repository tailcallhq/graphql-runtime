---
identity: true
---

# test-batching-group-by

```graphql @config
schema @server(port: 4000) @upstream(batch: {delay: 1, headers: [], maxSize: 1000}) {
  query: Query
}

type Post {
  body: String
  id: Int
  title: String
  user: User @http(url: "http://abc.com/users", batchKey: ["id"], query: [{key: "id", value: "{{.value.userId}}"}])
  userId: Int!
}

type Query {
  posts: [Post] @http(url: "http://abc.com/posts?id=1&id=11")
}

type User {
  id: Int
  name: String
}
```
