```json @config
{
  "inputs": [
    {
      "curl": {
        "src": "http://jsonplaceholder.typicode.com/users",
        "fieldName": "users"
      }
    },
    {
      "proto": {
        "src": "tailcall-fixtures/fixtures/protobuf/news.proto",
        "url": "http://localhost:50051"
      }
    }
  ],
  "preset": {
    "mergeType": 1.0,
    "inferTypeNames": true,
    "treeShake": true
  },
  "output": {
    "path": "./output.graphql",
    "format": "graphQL"
  },
  "schema": {
    "query": "Query"
  }
}
```
