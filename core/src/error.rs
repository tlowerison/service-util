use crate::{env::EnvError, InternalError};
use async_backtrace::{backtrace, Location};
use hyper::StatusCode;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[cfg(feature = "async-graphql-4")]
use async_graphql_4 as async_graphql;
#[cfg(feature = "async-graphql-5")]
use async_graphql_5 as async_graphql;
#[cfg(feature = "async-graphql-6")]
use async_graphql_6 as async_graphql;

#[cfg(feature = "client")]
use crate::BaseClientError;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Error {
    pub status_code: StatusCode,
    #[derivative(Debug = "ignore")]
    pub msg: Option<String>,
    #[derivative(Debug = "ignore")]
    pub details: Option<Arc<InternalError>>,
    #[derivative(Debug = "ignore")]
    pub backtrace: Option<Box<[Location]>>,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let Error { status_code, msg, .. } = &self;

        #[cfg(feature = "log_error")]
        {
            let Error { details, backtrace, .. } = &self;
            let mut log_err_msg = format!(" - status_code: {status_code}");
            if let Some(msg) = msg.as_ref() {
                log_err_msg = format!("{log_err_msg}\n - msg: {msg}");
            }
            if let Some(details) = details.as_ref() {
                log_err_msg = format!("{log_err_msg}\n - details: {details}");
            }
            if let Some(backtrace) = backtrace.as_ref() {
                let backtrace = backtrace.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
                log_err_msg = format!("{log_err_msg}\n - backtrace:\n{backtrace}");
            }

            tracing::error!("{log_err_msg}");
        }

        if let Some(canonical_reason) = status_code.canonical_reason() {
            write!(f, "{canonical_reason}")?;
            if let Some(msg) = msg.as_ref() {
                write!(f, ": {msg}")?;
            }
        } else if let Some(msg) = msg.as_ref() {
            write!(f, "{msg}")?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn init(
        status_code: impl Into<StatusCode>,
        msg: impl Into<Option<String>>,
        details: impl Into<Option<String>>,
    ) -> Self {
        Self {
            status_code: status_code.into(),
            msg: msg.into(),
            details: details.into().map(InternalError::msg).map(Arc::new),
            backtrace: backtrace(),
        }
    }

    pub fn init_with_backtrace(
        status_code: impl Into<StatusCode>,
        msg: impl Into<Option<String>>,
        details: impl Into<Option<String>>,
        backtrace: impl Into<Option<Box<[Location]>>>,
    ) -> Self {
        Self {
            status_code: status_code.into(),
            msg: msg.into(),
            details: details.into().map(InternalError::msg).map(Arc::new),
            backtrace: backtrace.into(),
        }
    }

    #[framed]
    pub fn new(status_code: impl Into<StatusCode>) -> Self {
        Error::init(status_code, None, None)
    }

    #[framed]
    pub fn status_map<E: Display>(status_code: impl Into<StatusCode>) -> Box<dyn Fn(E) -> Self> {
        let status_code = status_code.into();
        Box::new(move |err: E| Error::init(status_code, None, format!("{err}")))
    }

    #[framed]
    pub fn msg(status_code: impl Into<StatusCode>, err: impl Display) -> Self {
        Error::init(status_code, format!("{err}"), None)
    }

    #[framed]
    pub fn bad_request() -> Self {
        Error::init(StatusCode::BAD_REQUEST, None, None)
    }

    #[framed]
    pub fn default_msg(err: impl Display) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, format!("{err}"), None)
    }

    #[framed]
    pub fn bad_request_msg(err: impl Display) -> Self {
        Error::init(StatusCode::BAD_REQUEST, format!("{err}"), None)
    }

    #[framed]
    pub fn details(status_code: impl Into<StatusCode>, err: impl Display) -> Self {
        Error::init(status_code, None, format!("{err}"))
    }

    #[framed]
    pub fn default_details(err: impl Display) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }

    #[framed]
    pub fn default_msg_and_details(msg: impl Display, details: impl Display) -> Self {
        Error::init(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{msg}"),
            format!("{details}"),
        )
    }

    #[framed]
    pub fn bad_request_details(err: impl Display) -> Self {
        Error::init(StatusCode::BAD_REQUEST, None, format!("{err}"))
    }

    #[cfg(any(
        feature = "async-graphql-4",
        feature = "async-graphql-5",
        feature = "async-graphql-6"
    ))]
    pub fn graphql(self) -> async_graphql::Error {
        use async_graphql::ErrorExtensions;
        async_graphql::Error::new(self.msg.map(std::borrow::Cow::from).unwrap_or_else(|| {
            self.status_code
                .canonical_reason()
                .map(std::borrow::Cow::from)
                .unwrap_or_default()
        }))
        .extend_with(|_, extensions| extensions.set("status", self.status_code.as_u16()))
    }
}

impl Default for Error {
    #[framed]
    fn default() -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, None)
    }
}

impl From<EnvError> for Error {
    #[framed]
    fn from(err: EnvError) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }
}

impl From<InternalError> for Error {
    #[framed]
    fn from(err: InternalError) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            msg: None,
            details: Some(Arc::new(err)),
            backtrace: backtrace(),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    #[framed]
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }
}

impl From<(StatusCode, String)> for Error {
    fn from((status_code, msg): (StatusCode, String)) -> Self {
        Self::msg(status_code, msg)
    }
}

#[cfg(all(feature = "server", feature = "axum-05"))]
impl axum_05::response::IntoResponse for Error {
    fn into_response(self) -> axum_05::response::Response {
        let body = match self.msg {
            Some(msg) => axum_05::body::boxed(axum_05::body::Full::from(msg)),
            None => axum_05::body::boxed(axum_05::body::Empty::new()),
        };

        axum_05::response::Response::builder()
            .status(self.status_code)
            .body(body)
            .unwrap()
    }
}

#[cfg(all(feature = "server", feature = "axum-06"))]
impl axum_06::response::IntoResponse for Error {
    fn into_response(self) -> axum_06::response::Response {
        let body = match self.msg {
            Some(msg) => axum_06::body::boxed(axum_06::body::Full::from(msg)),
            None => axum_06::body::boxed(axum_06::body::Empty::new()),
        };

        axum_06::response::Response::builder()
            .status(self.status_code)
            .body(body)
            .unwrap()
    }
}

#[cfg(feature = "grpc")]
impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        if error.status_code.is_server_error() {
            return tonic::Status::new(tonic::Code::Internal, format!("{error}"));
        }
        tonic::Status::new(tonic::Code::Unknown, format!("{error}"))
    }
}

#[cfg(feature = "client")]
impl From<BaseClientError> for Error {
    #[framed]
    fn from(base_client_error: BaseClientError) -> Self {
        match base_client_error {
            BaseClientError::BodyTooLarge => Self::default(),
            BaseClientError::InvalidUri(invalid_uri) => Self::default_details(invalid_uri),
            BaseClientError::NetworkError(err) => Self::default_details(err),
            BaseClientError::RequestBodyBuild(err) => Self::default_details(err),
            BaseClientError::RequestBodySerialization(err) => Self::default_details(err),
            BaseClientError::RequestParamsSerialization(err) => Self::default_details(err),
            BaseClientError::Response { status, message } => Self::details(status, message),
            BaseClientError::ResponseBodyDeserialization(err) => Self::default_details(err),
            BaseClientError::ResponseBodyInvalidCharacter(err) => Self::default_details(err),
        }
    }
}

#[cfg(feature = "client")]
impl<Body> From<hyper::Response<Body>> for Error {
    default fn from(response: hyper::Response<Body>) -> Self {
        let status = response.status();
        if !(status.is_client_error() || status.is_server_error()) {
            unimplemented!();
        }

        Error::init(status, None, None)
    }
}

#[cfg(feature = "client")]
impl From<hyper::Response<Vec<u8>>> for Error {
    #[framed]
    fn from(response: hyper::Response<Vec<u8>>) -> Self {
        let status = response.status();
        if !(status.is_client_error() || status.is_server_error()) {
            unimplemented!();
        }

        let details = std::str::from_utf8(response.body().as_slice()).map(String::from).ok();

        Error::init(status, None, details)
    }
}

#[cfg(feature = "db")]
pub use db::*;
#[cfg(feature = "db")]
mod db {
    use super::*;

    use async_backtrace::backtrace;
    use diesel::result::DatabaseErrorKind;
    use diesel_util::{DbEntityError, TxCleanupError};
    use std::fmt::Display;

    impl<E: Display> From<DbEntityError<E>> for Error {
        fn from(value: DbEntityError<E>) -> Self {
            Self::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{value}"))
        }
    }

    impl From<diesel::result::Error> for Error {
        #[framed]
        fn from(err: diesel::result::Error) -> Self {
            match &err {
                diesel::result::Error::InvalidCString(_) => {
                    Error::init(StatusCode::BAD_REQUEST, None, format!("{err}"))
                }
                diesel::result::Error::DatabaseError(kind, _) => match kind {
                    DatabaseErrorKind::UniqueViolation => Error::init(StatusCode::BAD_REQUEST, None, format!("{err}")),
                    DatabaseErrorKind::ForeignKeyViolation => {
                        Error::init(StatusCode::BAD_REQUEST, None, format!("{err}"))
                    }
                    DatabaseErrorKind::UnableToSendCommand => {
                        Error::init(StatusCode::BAD_REQUEST, None, format!("{err}"))
                    }
                    DatabaseErrorKind::ReadOnlyTransaction => {
                        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                    }
                    DatabaseErrorKind::NotNullViolation => Error::init(StatusCode::BAD_REQUEST, None, format!("{err}")),
                    DatabaseErrorKind::CheckViolation => Error::init(StatusCode::BAD_REQUEST, None, format!("{err}")),
                    DatabaseErrorKind::ClosedConnection => {
                        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                    }
                    _ => Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}")),
                },
                diesel::result::Error::NotFound => Error::init(StatusCode::BAD_REQUEST, None, format!("{err}")),
                diesel::result::Error::QueryBuilderError(_) => {
                    Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                }
                diesel::result::Error::DeserializationError(_) => {
                    Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                }
                diesel::result::Error::SerializationError(_) => {
                    Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                }
                diesel::result::Error::AlreadyInTransaction => {
                    Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                }
                diesel::result::Error::NotInTransaction => {
                    Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
                }
                _ => Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}")),
            }
        }
    }

    impl From<TxCleanupError> for Error {
        #[framed]
        fn from(err: TxCleanupError) -> Self {
            Self::init_with_backtrace(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                format!("{}", err.source),
                err.backtrace,
            )
        }
    }

    impl From<Error> for TxCleanupError {
        #[framed]
        fn from(err: Error) -> Self {
            Self {
                source: InternalError::msg(err),
                backtrace: backtrace(),
            }
        }
    }
}
