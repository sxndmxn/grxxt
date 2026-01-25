//! greetd IPC communication module

use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{AuthMessageType, ErrorType, Request, Response};
use std::env;
use std::os::unix::net::UnixStream;

pub struct GreetdClient {
    stream: UnixStream,
}

#[derive(Debug)]
pub enum AuthError {
    ConnectionFailed(String),
    ProtocolError(String),
    AuthFailed(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            AuthError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            AuthError::AuthFailed(msg) => write!(f, "{}", msg),
        }
    }
}

impl GreetdClient {
    pub fn connect() -> Result<Self, AuthError> {
        let socket_path = env::var("GREETD_SOCK")
            .map_err(|_| AuthError::ConnectionFailed("GREETD_SOCK not set".into()))?;

        let stream = UnixStream::connect(&socket_path)
            .map_err(|e| AuthError::ConnectionFailed(e.to_string()))?;

        Ok(Self { stream })
    }

    fn send(&mut self, request: Request) -> Result<(), AuthError> {
        request
            .write_to(&mut self.stream)
            .map_err(|e| AuthError::ProtocolError(e.to_string()))
    }

    fn receive(&mut self) -> Result<Response, AuthError> {
        Response::read_from(&mut self.stream).map_err(|e| AuthError::ProtocolError(e.to_string()))
    }

    pub fn create_session(&mut self, username: &str) -> Result<(), AuthError> {
        self.send(Request::CreateSession {
            username: username.to_string(),
        })?;

        match self.receive()? {
            Response::Success => Ok(()),
            Response::AuthMessage { .. } => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(AuthError::AuthFailed(format_error(
                error_type,
                &description,
            ))),
        }
    }

    pub fn post_auth_response(&mut self, response: Option<String>) -> Result<AuthState, AuthError> {
        self.send(Request::PostAuthMessageResponse { response })?;

        match self.receive()? {
            Response::Success => Ok(AuthState::Done),
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => match auth_message_type {
                AuthMessageType::Visible => Ok(AuthState::NeedInput(auth_message)),
                AuthMessageType::Secret => Ok(AuthState::NeedSecret(auth_message)),
                AuthMessageType::Info => Ok(AuthState::Info(auth_message)),
                AuthMessageType::Error => Ok(AuthState::Error(auth_message)),
            },
            Response::Error {
                error_type,
                description,
            } => Err(AuthError::AuthFailed(format_error(
                error_type,
                &description,
            ))),
        }
    }

    pub fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError> {
        self.send(Request::StartSession { cmd, env: vec![] })?;

        match self.receive()? {
            Response::Success => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(AuthError::AuthFailed(format_error(
                error_type,
                &description,
            ))),
            _ => Err(AuthError::ProtocolError("Unexpected response".into())),
        }
    }

    #[allow(dead_code)]
    pub fn cancel_session(&mut self) -> Result<(), AuthError> {
        self.send(Request::CancelSession)?;
        match self.receive()? {
            Response::Success => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(AuthError::AuthFailed(format_error(
                error_type,
                &description,
            ))),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AuthState {
    NeedInput(String),
    NeedSecret(String),
    Info(String),
    Error(String),
    Done,
}

fn format_error(error_type: ErrorType, description: &str) -> String {
    match error_type {
        ErrorType::AuthError => {
            if description.is_empty() {
                "Authentication failed".to_string()
            } else {
                description.to_string()
            }
        }
        ErrorType::Error => description.to_string(),
    }
}

/// Perform full authentication flow
pub fn authenticate(username: &str, password: &str, session_cmd: &str) -> Result<(), AuthError> {
    let mut client = GreetdClient::connect()?;

    // Create session for user
    client.create_session(username)?;

    // Send password
    let state = client.post_auth_response(Some(password.to_string()))?;

    match state {
        AuthState::Done => {
            // Start the session
            let cmd: Vec<String> =
                shell_words::split(session_cmd).unwrap_or_else(|_| vec![session_cmd.to_string()]);
            client.start_session(cmd)?;
            Ok(())
        }
        AuthState::Error(msg) => Err(AuthError::AuthFailed(msg)),
        _ => Err(AuthError::ProtocolError("Unexpected auth state".into())),
    }
}
