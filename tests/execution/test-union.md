---
identity: true
---

# test-union

```graphql @config
schema @server @upstream(baseURL: "http://jsonplacheholder.typicode.com") {
  query: Query
}

scalar Baz

union FooBar = Bar | Foo

type Bar {
  bar: String
}

type Foo {
  foo: String
}

type Query {
  foo: FooBar @http(path: "/foo")
}
```
