// SPDX-License-Identifier: Apache-2.0

//! Configuration for a WASI application in an Enarx Keep
//!
#![doc = include_str!("../README.md")]
#![doc = include_str!("../Enarx_toml.md")]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![warn(rust_2018_idioms)]

use std::{collections::HashMap, ops::Deref};

use serde::{de::Error as _, Deserialize, Deserializer};
use url::Url;

const fn default_port() -> u16 {
    443
}

fn default_addr() -> String {
    "::".into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// Name of a file descriptor
///
/// This is used to export a list of file descriptor names in the `FD_NAMES` environment variable.
pub struct FileName(String);

impl From<String> for FileName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for FileName {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl Deref for FileName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for FileName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;

        if name.contains(':') {
            return Err(D::Error::custom("invalid value for `name` contains ':'"));
        }

        Ok(Self(name))
    }
}

/// The configuration for an Enarx WASI application
///
/// This struct can be used with any serde deserializer.
///
/// # Examples
///
/// ```
/// extern crate toml;
/// use enarx_config::EnarxConfig;
/// const CONFIG: &str = r#"
/// [[files]]
/// name = "LISTEN"
/// kind = "listen"
/// prot = "tls"
/// port = 12345
/// "#;
///
/// let config: EnarxConfig = toml::from_str(CONFIG).unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EnarxConfig {
    /// The environment variables to provide to the application
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// The arguments to provide to the application
    #[serde(default)]
    pub args: Vec<String>,

    /// The array of pre-opened file descriptors
    #[serde(default)]
    pub files: Vec<File>,

    /// An optional Steward URL
    #[serde(default)]
    pub steward: Option<Url>,
}

impl Default for EnarxConfig {
    fn default() -> Self {
        let files = vec![
            File::Stdin { name: None },
            File::Stdout { name: None },
            File::Stderr { name: None },
        ];

        Self {
            env: HashMap::new(),
            args: vec![],
            files,
            steward: None, // TODO: Default to a deployed Steward instance
        }
    }
}

/// Parameters for a pre-opened file descriptor
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind")]
pub enum File {
    /// file descriptor to `/dev/null`
    #[serde(rename = "null")]
    Null {
        /// name of the file descriptor
        name: Option<FileName>,
    },

    /// file descriptor to stdin
    #[serde(rename = "stdin")]
    Stdin {
        /// name of the file descriptor
        name: Option<FileName>,
    },

    /// file descriptor to stdout
    #[serde(rename = "stdout")]
    Stdout {
        /// name of the file descriptor
        name: Option<FileName>,
    },

    /// file descriptor to stderr
    #[serde(rename = "stderr")]
    Stderr {
        /// name of the file descriptor
        name: Option<FileName>,
    },

    /// file descriptor to a TCP listen socket
    #[serde(rename = "listen")]
    Listen {
        /// name of the file descriptor
        name: FileName,

        /// address to listen on
        #[serde(default = "default_addr")]
        addr: String,

        /// port to listen on
        #[serde(default = "default_port")]
        port: u16,

        /// protocol to use
        #[serde(default)]
        prot: Protocol,
    },

    /// file descriptor to a TCP stream socket
    #[serde(rename = "connect")]
    Connect {
        /// name of the file descriptor
        name: Option<FileName>,

        /// host address to connect to
        host: String,

        /// port to connect to
        #[serde(default = "default_port")]
        port: u16,

        /// protocol to use
        #[serde(default)]
        prot: Protocol,
    },
}

impl File {
    /// get the name for a file descriptor
    pub fn name(&self) -> &str {
        match self {
            Self::Null { name } => name.as_deref().unwrap_or("null"),
            Self::Stdin { name } => name.as_deref().unwrap_or("stdin"),
            Self::Stdout { name } => name.as_deref().unwrap_or("stdout"),
            Self::Stderr { name } => name.as_deref().unwrap_or("stderr"),
            Self::Listen { name, .. } => name,
            Self::Connect { name, host, .. } => name.as_deref().unwrap_or(host),
        }
    }
}

/// Protocol to use for a connection
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Protocol {
    /// transparently wrap the TCP connection with the TLS protocol
    #[serde(rename = "tls")]
    Tls,

    /// normal TCP connection
    #[serde(rename = "tcp")]
    Tcp,
}

impl Default for Protocol {
    fn default() -> Self {
        Self::Tls
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const CONFIG: &str = r#"
        [[files]]
        kind = "stdin"

        [[files]]
        name = "X"
        kind = "listen"
        prot = "tcp"
        port = 9000

        [[files]]
        kind = "stdout"

        [[files]]
        kind = "null"

        [[files]]
        kind = "stderr"

        [[files]]
        kind = "connect"
        host = "example.com"
    "#;

    #[test]
    fn values() {
        let cfg: EnarxConfig = toml::from_str(CONFIG).unwrap();

        assert_eq!(
            cfg.files,
            vec![
                File::Stdin { name: None },
                File::Listen {
                    name: "X".into(),
                    port: 9000,
                    prot: Protocol::Tcp,
                    addr: default_addr()
                },
                File::Stdout { name: None },
                File::Null { name: None },
                File::Stderr { name: None },
                File::Connect {
                    name: None,
                    port: default_port(),
                    prot: Protocol::Tls,
                    host: "example.com".into(),
                },
            ]
        );
    }

    #[test]
    fn names() {
        let cfg: EnarxConfig = toml::from_str(CONFIG).unwrap();

        assert_eq!(
            vec!["stdin", "X", "stdout", "null", "stderr", "example.com"],
            cfg.files.iter().map(|f| f.name()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn invalid_name() {
        const CONFIG: &str = r#"
        [[files]]
        name = "test:"
        kind = "null"
        "#;

        let err = toml::from_str::<EnarxConfig>(CONFIG).unwrap_err();
        assert_eq!(err.line_col(), Some((1, 8)));
        assert_eq!(
            err.to_string(),
            "invalid value for `name` contains ':' for key `files` at line 2 column 9"
        );
    }
}
