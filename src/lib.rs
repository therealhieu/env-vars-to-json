#![doc = include_str!("../README.md")]

use core::panic;
use std::env;

use derive_builder::Builder;
use serde_json::{json, Number, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("serde_json error: {0}")]
    SerdeJson(serde_json::Error),

    #[error("Encountered error while parsing environment variables: {0}")]
    Internal(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::Internal(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

#[derive(Debug, Builder)]
pub struct EnvVarsToJson {
    #[builder(default, setter(into, strip_option))]
    pub prefix: Option<String>,

    #[builder(default = "String::from(\"__\")", setter(into))]
    pub separator: String,
}

impl Default for EnvVarsToJson {
    fn default() -> Self {
        Self {
            prefix: None,
            separator: "__".to_string(),
        }
    }
}

type ArrayIndex = usize;

#[derive(Debug)]
pub enum PartValue {
    Object(Value),
    ArrayItem(ArrayItem),
}

impl PartValue {
    pub fn into_json_value(self) -> Value {
        match self {
            Self::Object(value) => value,
            Self::ArrayItem(item) => item.into_array_value(),
        }
    }
}

#[derive(Debug)]
pub struct ArrayItem {
    pub index: ArrayIndex,
    pub value: Value,
}

impl ArrayItem {
    pub fn new(index: ArrayIndex, value: Value) -> Self {
        Self { index, value }
    }

    pub fn into_array_value(self) -> Value {
        if self.index == 0 {
            return Value::Array(vec![self.value]);
        }

        let mut arr = vec![Value::Null; self.index];
        arr.push(self.value);
        Value::Array(arr)
    }
}

#[derive(Debug, Clone)]
pub enum JsonIndex {
    String(String),
    Usize(usize),
}

impl JsonIndex {
    pub fn from_vec(vec: Vec<&str>) -> Vec<Self> {
        vec.into_iter().map(Self::from).collect()
    }
}

impl From<&str> for JsonIndex {
    fn from(s: &str) -> Self {
        if let Ok(number) = s.parse::<usize>() {
            Self::Usize(number)
        } else {
            Self::String(s.to_string())
        }
    }
}

impl From<String> for JsonIndex {
    fn from(s: String) -> Self {
        if let Ok(number) = s.parse::<usize>() {
            Self::Usize(number)
        } else {
            Self::String(s)
        }
    }
}

impl From<&String> for JsonIndex {
    fn from(s: &String) -> Self {
        if let Ok(number) = s.parse::<usize>() {
            Self::Usize(number)
        } else {
            Self::String(s.clone())
        }
    }
}

impl EnvVarsToJson {
    pub fn builder() -> EnvVarsToJsonBuilder {
        EnvVarsToJsonBuilder::default()
    }

    /// Examples:
    ///
    /// Given environemnt variables, with prefix `PREFIX` and separator `__`:
    /// ```bash
    /// export PREFIX__INT_LIST__0=1
    /// export PREFIX__INT_LIST__1=2
    /// export PREFIX__STRUCT__INT=1
    /// export PREFIX__STRUCT__STRING=string
    /// export PREFIX__STRUCT__BOOL_LIST__0=true
    /// export PREFIX__STRUCT__BOOL_LIST__1=false
    /// ```
    ///
    /// Ouptut json:
    /// ```json
    /// {
    ///   "int_list": [1, 2],
    ///   "struct": {
    ///     "int": 1,'
    ///     "string": "string",
    ///     "bool_list": [true, false]
    ///   }
    /// }
    /// ```
    pub fn parse_from_env(&self) -> Result<serde_json::Value, Error> {
        self.parse_iter(env::vars())
    }

    /// Preprocess environment variables by filtering and sorting them
    fn preprocess_vars(
        &self,
        vars: impl Iterator<Item = (String, String)>,
    ) -> Result<Vec<(String, String)>, Error> {
        let mut vars = if let Some(prefix) = &self.prefix {
            vars.filter(|(key, _)| key.starts_with(prefix))
                .map(|(key, value)| {
                    Ok((
                        key.strip_prefix(prefix)
                            .ok_or_else(|| format!("key {key} does not match prefix {prefix}"))?
                            .to_string(),
                        value,
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            vars.collect::<Vec<_>>()
        };

        // Sort in reverse order to ensure that the longest keys are processed first
        vars.sort_by(|(key_a, _), (key_b, _)| key_b.cmp(key_a));

        Ok(vars)
    }

    /// Parse iterator of String tuples into json
    pub fn parse_iter(&self, vars: impl Iterator<Item = (String, String)>) -> Result<Value, Error> {
        let vars = self.preprocess_vars(vars)?;
        let mut json = json!({});

        for (key, env_value) in vars {
            let key_parts = key
                .split(&self.separator)
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>();

            let env_value = if let Ok(value) = env_value.parse::<i64>() {
                Value::Number(value.into())
            } else if let Ok(value) = env_value.parse::<f64>() {
                Value::Number(Number::from_f64(value).ok_or("Failed to parse float")?)
            } else if let Ok(value) = env_value.parse::<bool>() {
                Value::Bool(value)
            } else {
                Value::String(env_value)
            };

            if key_parts.len() == 1 {
                // Raise error if part is a number
                if key_parts[0].parse::<usize>().is_ok() {
                    return Err("First key part cannot be a number".into());
                }

                json[key_parts[0].as_str()] = env_value.clone();
                continue;
            }

            // Reverse key parts to iterate from the bottom up
            // Index starts at len - 1
            let mut part_value = PartValue::Object(env_value);

            for (i, part) in key_parts.iter().cloned().enumerate().rev() {
                // Query json, check if part exists in json
                let indices = key_parts[..i + 1]
                    .iter()
                    .cloned()
                    .map(JsonIndex::from)
                    .collect::<Vec<_>>();

                // If part exists, replace part value in json with env var value
                if let Some(curr_part_value) = Self::json_get_mut(&mut json, &indices) {
                    match part_value {
                        PartValue::Object(value) => match curr_part_value {
                            Value::Object(obj) => {
                                let (k, v) = value
                                    .as_object()
                                    .ok_or(format!("Expected object, got: {:?}", value))?
                                    .iter()
                                    .next()
                                    .unwrap();
                                obj.insert(k.clone(), v.clone());
                            }
                            Value::Null => *curr_part_value = value,
                            _ => panic!("Unexpected value: {:?}", curr_part_value),
                        },
                        PartValue::ArrayItem(array_item) => {
                            curr_part_value
                                .as_array_mut()
                                .ok_or("Expected array")?
                                .insert(array_item.index, array_item.value);
                        }
                    };
                    break;
                }

                if indices.len() == 1 {
                    json.as_object_mut().ok_or("Expected object")?.insert(
                        part.to_string().to_lowercase(),
                        part_value.into_json_value(),
                    );
                    break;
                }

                // If not, we create an Object or Array dependingo on part type (string or usize)
                // If part is string, create an Object
                if part.parse::<usize>().is_err() {
                    part_value = PartValue::Object(json! {{ part: part_value.into_json_value() }});
                    continue;
                }

                // If part is usize,  create an Array
                let index = part.parse::<usize>().expect("This should never fail");
                part_value =
                    PartValue::ArrayItem(ArrayItem::new(index, part_value.into_json_value()));
            }
        }

        Ok(json)
    }

    /// Get mutable reference to json value at indices
    pub fn json_get_mut<'a>(
        json: &'a mut Value,
        indices: &'a [JsonIndex],
    ) -> Option<&'a mut Value> {
        let mut json = json;

        for index in indices {
            match index {
                JsonIndex::String(key) => {
                    json = json.get_mut(key)?;
                }
                JsonIndex::Usize(index) => {
                    json = json.get_mut(index)?;
                }
            }
        }

        Some(json)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rstest::rstest;
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct TestCase<'a> {
        prefix: Option<&'a str>,
        separator: &'a str,
        env_vars: HashMap<&'a str, &'a str>,
        expected: String,
    }

    impl TestCase<'_> {
        pub fn from_yaml(yaml: &'static str) -> Self {
            serde_yaml::from_str(yaml).expect("failed to parse yaml")
        }

        pub fn vars(&self) -> impl Iterator<Item = (String, String)> + '_ {
            self.env_vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
        }

        pub fn set_vars(&self) {
            for (k, v) in self.env_vars.iter() {
                std::env::set_var(k, v);
            }
        }

        pub fn assert(&self, actual: &serde_json::Value) {
            let expected = serde_json::from_str::<serde_json::Value>(&self.expected)
                .expect("failed to parse expected json");
            assert_eq!(actual, &expected);
        }
    }

    impl From<&TestCase<'_>> for EnvVarsToJson {
        fn from(test_case: &TestCase) -> Self {
            let mut builder = EnvVarsToJson::builder();
            builder.separator(test_case.separator);

            if let Some(prefix) = test_case.prefix {
                builder.prefix(prefix);
            }

            builder.build().expect("failed to build EnvVarsToJson")
        }
    }

    #[rstest]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__INT_LIST__1: "2"
        expected: |
            {
                "int_list": [null, 2]
            }
        "#
    )]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__STRUCT__INT: "1"
            PREFIX__STRUCT__STRING: "string"
        expected: |
            {
                "struct": {
                    "int": 1,
                    "string": "string"
                }
            }
        "#
    )]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__STRUCT__INT: "1"
            PREFIX__STRUCT__STRING: "string"
            PREFIX__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__BOOL_LIST__1: "false"
        expected: |
            {
                "struct": {
                    "int": 1,
                    "string": "string",
                    "bool_list": [true, false]
                }
            }
    "#
    )]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__INT_LIST__0: "1"
            PREFIX__INT_LIST__1: "2"
            PREFIX__STRUCT__INT: "1"
            PREFIX__STRUCT__STRING: "string"
            PREFIX__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__BOOL_LIST__1: "false"
        expected: |
          {
            "int_list": [1, 2],
            "struct": {
              "int": 1,
              "string": "string",
              "bool_list": [true, false]
            }
          }
    "#
    )]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__INT_LIST__0: "1"
            PREFIX__INT_LIST__1: "2"
            PREFIX__STRUCT__INT: "1"
            PREFIX__STRUCT__STRING: "string"
            PREFIX__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__BOOL_LIST__1: "false"
            PREFIX__STRUCT__STRUCT__INT: "1"
            PREFIX__STRUCT__STRUCT__STRING: "string"
            PREFIX__STRUCT__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__STRUCT__BOOL_LIST__1: "false"
            PREFIX__BOOL_LIST__3: "true"
            PREFIX__STRUCT__FLOAT: "1.1"
            PREFIX__BOOL_LIST__0: "false"
            PREFIX__STRING_LIST__0: "string0"
        expected: |
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
    "#
    )]
    fn test_parse_iter(#[case] test_yaml: &'static str) -> Result<(), Error> {
        let test_case = TestCase::from_yaml(test_yaml);
        let env_vars_to_json = EnvVarsToJson::from(&test_case);
        let actual = env_vars_to_json.parse_iter(test_case.vars())?;
        test_case.assert(&actual);

        Ok(())
    }

    #[rstest]
    #[case(
        r#"
        prefix: PREFIX__
        separator: "__"
        env_vars:
            PREFIX__INT_LIST__0: "1"
            PREFIX__INT_LIST__1: "2"
            PREFIX__STRUCT__INT: "1"
            PREFIX__STRUCT__STRING: "string"
            PREFIX__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__BOOL_LIST__1: "false"
            PREFIX__STRUCT__STRUCT__INT: "1"
            PREFIX__STRUCT__STRUCT__STRING: "string"
            PREFIX__STRUCT__STRUCT__BOOL_LIST__0: "true"
            PREFIX__STRUCT__STRUCT__BOOL_LIST__1: "false"
            PREFIX__BOOL_LIST__3: "true"
            PREFIX__STRUCT__FLOAT: "1.1"
            PREFIX__BOOL_LIST__0: "false"
            PREFIX__STRING_LIST__0: "string0"
        expected: |
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
    "#
    )]
    fn test_parse_from_env(#[case] test_yaml: &'static str) -> Result<(), Error> {
        let test_case = TestCase::from_yaml(test_yaml);
        test_case.set_vars();
        let env_vars_to_json = EnvVarsToJson::from(&test_case);
        let actual = env_vars_to_json.parse_from_env()?;
        println!("{}", serde_json::to_string_pretty(&actual).unwrap());
        test_case.assert(&actual);

        Ok(())
    }
}
