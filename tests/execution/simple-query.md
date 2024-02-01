# Simple query

#### server:

```json
{
  "server": {},
  "upstream": {
    "baseURL": "http://jsonplaceholder.typicode.com"
  },
  "schema": {
    "query": "Query"
  },
  "types": {
    "Query": {
      "fields": {
        "firstUser": {
          "type": "User",
          "http": {
            "path": "/users/1"
          },
          "cache": null
        }
      },
      "cache": null
    },
    "User": {
      "fields": {
        "id": {
          "type": "Int",
          "cache": null
        },
        "name": {
          "type": "String",
          "cache": null
        }
      },
      "cache": null
    }
  }
}
```

#### mock:

```yml
- request:
    method: GET
    url: http://jsonplaceholder.typicode.com/users/1
    body: null
  response:
    status: 200
    body:
      id: 1
      name: foo
```

#### assert:

```yml
- method: POST
  url: http://localhost:8080/graphql
  body:
    query: query { firstUser { name } }
```
