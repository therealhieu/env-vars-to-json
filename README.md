# env-vars-to-json: Construct JSON from environment variables
[![CI](https://github.com/therealhieu/env-vars-to-json/actions/workflows/ci.yml/badge.svg)](https://github.com/therealhieu/env-vars-to-json/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/therealhieu/env-vars-to-json/branch/master/graph/badge.svg?token=BVA3LWO7HF)](https://codecov.io/gh/therealhieu/env-vars-to-json)
[![Crates.io](https://img.shields.io/crates/v/env_vars_to_json.svg)](https://crates.io/crates/env-vars-to-json)
[![Docs.rs](https://docs.rs/crate_name/badge.svg)](https://docs.rs/env_vars_to_json)


This crate provides methods to construct `serde_json::Value` from environment variables.

Examples:

Given environemnt variables, with prefix `PREFIX` and separator `__`:
```bash
export PREFIX__INT_LIST__0=1
export PREFIX__INT_LIST__1=2
export PREFIX__STRUCT__INT=1
export PREFIX__STRUCT__STRING=string
export PREFIX__STRUCT__BOOL_LIST__0=true
export PREFIX__STRUCT__BOOL_LIST__1=false
export PREFIX__STRUCT__STRUCT__INT=1
export PREFIX__STRUCT__STRUCT__STRING=string
export PREFIX__STRUCT__STRUCT__BOOL_LIST__0=true
export PREFIX__STRUCT__STRUCT__BOOL_LIST__1=false
export PREFIX__BOOL_LIST__3=true
export PREFIX__STRUCT__FLOAT=1.1
export PREFIX__BOOL_LIST__0=false
export PREFIX__STRING_LIST__0=string0
```

**Without default json**:
```rust,no_run
use serde_json::json;
use env_vars_to_json::Parser;

let json = Parser::default()
    .with_prefix("PREFIX")
    .with_separator("__")
    .parse_from_env()
    .expect("Failed to parse environment variables");

assert_eq!(json, json!(
  {
    "int_list": [1, 2],
    "struct": {
      "int": 1,
      "float": 1.1,
      "string": "string",
      "bool_list": [true, false],
      "struct": {
        "int": 1,
        "string": "string",
        "bool_list": [true, false]
      }
    },
    "bool_list": [false, null, null, true],
    "string_list": ["string0"]
  }
))
```

**With default json and filters (require feature `filter` enabled)**:
```rust,feature=filter,no_run
use serde_json::json;
use env_vars_to_json::Parser;

let json = Parser::default()
    .with_prefix("PREFIX")
    .with_separator("__")
    .with_include(&[".*STRUCT.*"])
    .with_exclude(&[".*INT_LIST.*", "PREFIX__BOOL_LIST.*"])
    .with_json(json!(
        {
          "float_list": [1.1],
          "string_list": ["a", "b"],
          "bool_list": [true, false]
        }
    ))
    .parse_from_env()
    .expect("Failed to parse environment variables");

assert_eq!(json, json!(
  {
      "float_list": [1.1],
      "struct": {
        "int": 1,
        "float": 1.1,
        "string": "string",
        "bool_list": [true, false],
        "struct": {
          "int": 1,
          "string": "string",
          "bool_list": [true, false]
        }
      },
      "bool_list": [true, false],
      "string_list": ["a", "b"]
    }  
));
```


## License
Licensed under either of
 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
 * MIT license
   ([LICENSE-MIT](LICENE-MIT) or <http://opensource.org/licenses/MIT)>

at your option.