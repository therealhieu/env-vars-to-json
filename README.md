# env-vars-to-json: Construct serde_json::Value from environment variables
## Introduction
This crate provides a method to construct `serde_json::Value` from environment variables.

Examples:

Given environemnt variables, with prefix `PREFIX` and separator `__`:
```bash
export PREFIX__INT_LIST__0=1
export PREFIX__INT_LIST__1=2
export PREFIX__STRUCT__INT=1
export PREFIX__STRUCT__STRING=string
export PREFIX__STRUCT__BOOL_LIST__0=true
export PREFIX__STRUCT__BOOL_LIST__1=false 
```

Ouptut json:
```json
{
  "int_list": [1, 2],
  "struct": {
    "int": 1,
    "string": "string",
    "bool_list": [true, false]
  }
}
```

Code:
```rust
use env_vars_to_json::EnvVarsToJson;

let json = EnvVarsToJson::builder()
    .prefix("PREFIX")
    .separator("__")
    .build()
    .expect("Failed to build EnvVarsToJson")
    .parse()
    .expect("Failed to parse environment variables");

println!("{}", json.to_string());
```