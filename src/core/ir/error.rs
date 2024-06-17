use std::fmt::Display;
use std::sync::Arc;

use async_graphql::{ErrorExtensions, Value as ConstValue};

use crate::core::auth;
use crate::core::error::{cache, http, worker, Error as CoreError};

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    IOError(String),

    GRPCError {
        grpc_code: i32,
        grpc_description: String,
        grpc_status_message: String,
        grpc_status_details: ConstValue,
    },

    APIValidationError(Vec<String>),

    // FIXME: Use specific error types instead of string
    Other(String),

    DeserializeError(String),

    AuthError(String),

    WorkerError(String),

    HttpError(String),

    CacheError(String),

    CoreError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::core::Errata::from(self.to_owned()).fmt(f)
    }
}

impl From<Error> for crate::core::Errata {
    fn from(value: Error) -> Self {
        use crate::core::Errata as CoreError;
        match value {
            Error::IOError(message) => CoreError::new("IO Error").description(message),
            Error::GRPCError {
                grpc_code,
                grpc_description,
                grpc_status_message,
                grpc_status_details,
            } => CoreError::new("GRPC Error")
                .description(grpc_description)
                .caused_by(vec![CoreError::new(
                    format!("code: {}, message: {}", grpc_code, grpc_status_message).as_str(),
                )])
                .description(grpc_status_details.to_string()),
            Error::APIValidationError(errors) => CoreError::new("API Validation Error")
                .caused_by(errors.iter().map(|e| CoreError::new(e)).collect::<Vec<_>>()),
            Error::Other(message) => CoreError::new("Evaluation Error").description(message),
            Error::DeserializeError(message) => {
                CoreError::new("Deserialization Error").description(message)
            }

            Error::AuthError(message) => {
                CoreError::new("Authentication Error").description(message)
            }

            Error::WorkerError(message) => CoreError::new("Worker Error").description(message),

            Error::HttpError(message) => CoreError::new("HTTP Error").description(message),

            Error::CacheError(message) => CoreError::new("Cache Error").description(message),

            Error::CoreError(message) => CoreError::new("Core Error").description(message),
        }
    }
}

impl ErrorExtensions for Error {
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(format!("{}", self)).extend_with(|_err, e| {
            if let Error::GRPCError {
                grpc_code,
                grpc_description,
                grpc_status_message,
                grpc_status_details,
            } = self
            {
                e.set("grpcCode", *grpc_code);
                e.set("grpcDescription", grpc_description);
                e.set("grpcStatusMessage", grpc_status_message);
                e.set("grpcStatusDetails", grpc_status_details.clone());
            }
        })
    }
}

impl From<auth::error::Error> for Error {
    fn from(value: auth::error::Error) -> Self {
        Error::AuthError(value.to_string())
    }
}

// Some dummy Implementation
impl From<worker::Error> for Error {
    fn from(value: worker::Error) -> Self {
        Error::WorkerError(value.to_string())
    }
}

// Some dummy Implementation
impl From<http::Error> for Error {
    fn from(value: http::Error) -> Self {
        Error::HttpError(value.to_string())
    }
}

impl From<http::Error> for Arc<Error> {
    fn from(value: http::Error) -> Self {
        Arc::new(Error::HttpError(value.to_string()))
    }
}

impl From<cache::Error> for Error {
    fn from(value: cache::Error) -> Self {
        Error::CacheError(value.to_string())
    }
}

impl From<CoreError> for Error {
    fn from(value: CoreError) -> Self {
        Error::CoreError(value.to_string())
    }
}

impl<'a> From<crate::core::valid::ValidationError<&'a str>> for Error {
    fn from(value: crate::core::valid::ValidationError<&'a str>) -> Self {
        Error::APIValidationError(
            value
                .as_vec()
                .iter()
                .map(|e| e.message.to_owned())
                .collect(),
        )
    }
}

impl From<Arc<Error>> for Error {
    fn from(error: Arc<Error>) -> Self {
        Error::WorkerError(error.to_string())
    }
}
