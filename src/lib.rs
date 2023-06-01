#![doc = include_str!("../README.md")]

use derive_builder::Builder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("serde_json error: {0}")]
    SerdeJson(serde_json::Error),
}

#[derive(Debug, Builder)]
pub struct EnvVarsToJson {
    pub prefix: Option<String>,
    pub separator: String,
}

impl EnvVarsToJson {
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
    ///     "int": 1,
    ///     "string": "string",
    ///     "bool_list": [true, false]
    ///   }
    /// }
    /// ```
    pub fn parse(&self) -> Result<serde_json::Value, Error> {
        todo!()
    }
}
