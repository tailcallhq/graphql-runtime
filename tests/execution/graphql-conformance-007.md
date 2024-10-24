---
skip: true
---

# Test named fragments.

TODO: Skipped because Tailcall does not send the whole query with the **fragments** to the remote server.

```graphql @config
schema @server(port: 8001, queryValidation: false, hostname: "0.0.0.0") @upstream(httpCache: 42) {
  query: Query
}

type Query {
  profiles(handles: [ID!]!): [Profile!]!
    @graphQL(url: "http://upstream/graphql", name: "profiles", args: [{key: "handles", value: "{{.args.handles}}"}])
}

interface Profile {
  id: ID!
  handle: String!
}

type User implements Profile {
  id: ID!
  handle: String!
  friends: Counter!
}

type Page implements Profile {
  id: ID!
  handle: String!
  likers: Counter!
}

type Counter {
  count: Int!
}
```

```yml @mock
- request:
    method: POST
    url: http://upstream/graphql
    textBody: '{ "query": "query { profiles(handles: [\"user-1\", \"user-2\"]) { handle ...userFragment ...pageFragment } } fragment userFragment on User { friends { count } } fargment pageFragment on Page { likers { count } }" }'
  expectedHits: 1
  response:
    status: 200
    body:
      data:
        profiles:
          - id: 1
            handle: user-1
            __typename: User
            friends:
              count: 2
          - id: 2
            handle: user-2
            __typename: Page
            likers:
              count: 4
```

```yml @test
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: |
      query {
        profiles(handles: ["user-1", "user-2"]) {
          handle
          ...userFragment
          ...pageFragment
        }
      }

      fragment userFragment on User {
        friends {
          count
        }
      }

      fragment pageFragment on Page {
        likers {
          count
        }
      }
```
