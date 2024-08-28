```json @config
{
  "inputs": [
    {
      "curl": {
        "src": "https://jsonplaceholder.typicode.com/users",
        "fieldName": "users"
      }
    },
    {
      "proto": {
        "src": "tailcall-fixtures/fixtures/protobuf/news.proto"
      }
    }
  ],
  "preset": {
    "mergeType": 1.0,
    "consolidateURL": 0.5,
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
