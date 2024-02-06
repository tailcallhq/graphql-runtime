# test-grpc-group-by

###### sdl error

#### server:

```graphql
schema
  @server(port: 8000, graphiql: true)
  @upstream(httpCache: true, batch: {delay: 10})
  @link(id: "news", src: "../../src/grpc/tests/news.proto", type: Protobuf) {
  query: Query
}

type Query {
  newsById(news: NewsInput!): News!
    @grpc(
      service: "news.NewsService"
      method: "GetMultipleNews"
      baseURL: "http://localhost:50051"
      body: "{{args.news}}"
      protoId: "news"
      groupBy: ["id"]
    )
}
input NewsInput {
  id: Int
  title: String
  body: String
  postImage: String
}
type NewsData {
  news: [News]!
}

type News {
  id: Int
  title: String
  body: String
  postImage: String
}
```
