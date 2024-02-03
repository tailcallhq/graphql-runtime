# n + 1 Request List

#### server:

```graphql
schema @upstream(baseURL: "http://example.com", batch: {delay: 1, maxSize: 1000}) {
  query: Query
}

type Query {
  foos: [Foo] @http(path: "/foos")
  bars: [Bar] @http(path: "/bars")
}

type Foo {
  id: Int!
  name: String!
  bar: Bar @http(path: "/bars", query: [{key: "fooId", value: "{{value.id}}"}], groupBy: ["fooId"])
}

type Bar {
  id: Int!
  fooId: Int!
  foo: [Foo] @http(path: "/foos", query: [{key: "id", value: "{{value.fooId}}"}], groupBy: ["id"])
}
```

#### mock:

```yml
- request:
    method: GET
    url: http://example.com/bars
    body: null
  response:
    status: 200
    body:
      - fooId: 1
        id: 1
      - fooId: 1
        id: 2
      - fooId: 2
        id: 3
      - fooId: 2
        id: 4
- request:
    method: GET
    url: http://example.com/foos?id=1&id=2
    body: null
  response:
    status: 200
    body:
      - id: 1
        name: foo1
      - id: 2
        name: foo2
```

#### assert:

```yml
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: query { bars { foo { id } fooId id } }
```
