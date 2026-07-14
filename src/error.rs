// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;

pub type Result<T> = std::result::Result<T, SqlsmithError>;

#[derive(Debug)]
pub enum SqlsmithError {
    AuthenticateFailure(String),
    BadArguments(String),
    TypeResolution(String),
    Io(std::io::Error),
    Http(reqwest::Error),
    Json(serde_json::Error),
    Message(String),
}

impl SqlsmithError {
    pub(crate) fn is_connection_error(&self) -> bool {
        matches!(
            self,
            SqlsmithError::Http(err)
                if err.is_connect() || err.is_timeout() || err.is_request()
        )
    }
}

impl fmt::Display for SqlsmithError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlsmithError::AuthenticateFailure(message) => {
                write!(f, "authentication failure: {message}")
            }
            SqlsmithError::BadArguments(message) => write!(f, "bad arguments: {message}"),
            SqlsmithError::TypeResolution(message) => write!(f, "type resolution error: {message}"),
            SqlsmithError::Io(err) => write!(f, "io error: {err}"),
            SqlsmithError::Http(err) => write!(f, "http error: {err}"),
            SqlsmithError::Json(err) => write!(f, "json error: {err}"),
            SqlsmithError::Message(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SqlsmithError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SqlsmithError::Io(err) => Some(err),
            SqlsmithError::Http(err) => Some(err),
            SqlsmithError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SqlsmithError {
    fn from(err: std::io::Error) -> Self {
        SqlsmithError::Io(err)
    }
}

impl From<reqwest::Error> for SqlsmithError {
    fn from(err: reqwest::Error) -> Self {
        SqlsmithError::Http(err)
    }
}

impl From<serde_json::Error> for SqlsmithError {
    fn from(err: serde_json::Error) -> Self {
        SqlsmithError::Json(err)
    }
}

impl From<String> for SqlsmithError {
    fn from(message: String) -> Self {
        SqlsmithError::Message(message)
    }
}

impl From<&str> for SqlsmithError {
    fn from(message: &str) -> Self {
        SqlsmithError::Message(message.to_string())
    }
}
