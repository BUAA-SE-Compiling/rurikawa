use err_derive::Error;
use std::fmt::Debug;
use tokio_tungstenite::tungstenite;

#[derive(Debug, Error)]
pub enum JobExecErr {
    #[error(display = "No such file: {}", _0)]
    NoSuchFile(String),

    #[error(display = "No such config: {}", _0)]
    NoSuchConfig(String),

    #[error(display = "IO error: {}", _0)]
    Io(#[error(source)] std::io::Error),

    #[error(display = "Web request error: {}", _0)]
    Request(#[error(source)] reqwest::Error),

    #[error(display = "Websocket error: {}", _0)]
    Ws(#[error(source, no_from)] tungstenite::Error),

    #[error(display = "JSON error: {}", _0)]
    Json(#[error(source)] serde_json::Error),

    #[error(display = "TOML deserialization error: {}", _0)]
    TomlDes(#[error(source)] toml::de::Error),

    #[error(display = "Build error: {}", _0)]
    Build(#[error(source)] crate::tester::BuildError),

    #[error(display = "Execution error: {}", _0)]
    Exec(#[error(source)] crate::tester::ExecError),

    #[error(display = "Job was cancelled")]
    Cancelled,

    #[error(display = "{:#}", _0)]
    Any(anyhow::Error),
}

impl From<tungstenite::error::Error> for JobExecErr {
    fn from(e: tungstenite::error::Error) -> Self {
        match e {
            tungstenite::Error::Io(e) => {
                Self::Any(anyhow::Error::new(e).context("error inside websocket"))
            }
            _ => JobExecErr::Ws(e),
        }
    }
}

macro_rules! anyhow_downcast_chain {
    ($e:expr, $($ty:ty),*) => {
        $(if $e.is::<$ty>(){
            let e = $e.downcast::<$ty>().unwrap();
            return e.into();
        })*
    };
}

impl From<anyhow::Error> for JobExecErr {
    fn from(e: anyhow::Error) -> Self {
        if e.chain().count() > 1 {
            tracing::warn!(
                "Context may be stripped during downcast. Logging error here:\n{:#}",
                e
            );
        }
        anyhow_downcast_chain!(
            e,
            crate::tester::BuildError,
            crate::tester::ExecError,
            std::io::Error,
            toml::de::Error,
            reqwest::Error
        );
        JobExecErr::Any(e)
    }
}

#[derive(Debug, Error)]
pub enum ClientConnectionErr {
    #[error(display = "Websocket error: {}", _0)]
    Ws(#[error(from)] tungstenite::Error),
    #[error(display = "Bad access token")]
    BadAccessToken,
    #[error(display = "Bad register token")]
    BadRegisterToken,
}
