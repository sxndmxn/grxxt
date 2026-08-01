//! greetd IPC communication module.
//!
//! The protocol is a native-endian `u32` byte length followed by a JSON request or response.
//! Frames are implemented locally so reads can be bounded before allocation and credential
//! buffers can be zeroized immediately after transmission.

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use zeroize::Zeroize;

const IPC_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FRAME_BYTES: usize = 128 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Request {
    CreateSession { username: String },
    PostAuthMessageResponse { response: Option<String> },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorType {
    Error,
    AuthError,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuthMessageType {
    Visible,
    Secret,
    Info,
    Error,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Response {
    Success,
    Error {
        error_type: ErrorType,
        description: String,
    },
    AuthMessage {
        auth_message_type: AuthMessageType,
        auth_message: String,
    },
}

pub struct GreetdClient {
    stream: UnixStream,
}

trait GreetdSession {
    fn create_session(&mut self, username: &str) -> Result<AuthState, AuthError>;
    fn post_auth_response(&mut self, response: Option<String>) -> Result<AuthState, AuthError>;
    fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError>;
    fn cancel_session(&mut self) -> Result<(), AuthError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Authentication failed")]
    AuthFailed,
    #[error("greetd request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid session command: {0}")]
    InvalidSessionCommand(String),
    #[error("Unsupported authentication prompt: {0}")]
    UnsupportedPrompt(String),
    #[error("{cause}; failed to cancel greetd session: {cleanup}")]
    SessionCleanupFailed {
        cause: Box<Self>,
        cleanup: Box<Self>,
    },
}

impl GreetdClient {
    pub fn connect() -> Result<Self, AuthError> {
        let socket_path = env::var("GREETD_SOCK")
            .map_err(|_| AuthError::ConnectionFailed("GREETD_SOCK not set".into()))?;

        let stream = UnixStream::connect(&socket_path)
            .map_err(|error| AuthError::ConnectionFailed(error.to_string()))?;
        stream
            .set_read_timeout(Some(IPC_TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(IPC_TIMEOUT)))
            .map_err(|error| {
                AuthError::ConnectionFailed(format!("failed to configure greetd socket: {error}"))
            })?;

        Ok(Self { stream })
    }

    fn send(&mut self, request: &Request) -> Result<(), AuthError> {
        write_request(&mut self.stream, request)
    }

    fn receive(&mut self) -> Result<Response, AuthError> {
        read_response(&mut self.stream)
    }
}

fn write_request(writer: &mut impl Write, request: &Request) -> Result<(), AuthError> {
    // Preallocate the full bounded frame so password-bearing JSON cannot leave
    // stale copies behind through Vec reallocations before it is zeroized.
    let mut body = Vec::with_capacity(MAX_FRAME_BYTES);
    let result = serde_json::to_writer(&mut body, request)
        .map_err(|error| AuthError::ProtocolError(error.to_string()))
        .and_then(|()| {
            if body.len() > MAX_FRAME_BYTES {
                return Err(AuthError::ProtocolError(format!(
                    "request exceeds {MAX_FRAME_BYTES} bytes"
                )));
            }

            let length = u32::try_from(body.len())
                .map_err(|error| AuthError::ProtocolError(error.to_string()))?;
            writer
                .write_all(&length.to_ne_bytes())
                .and_then(|()| writer.write_all(&body))
                .map_err(|error| AuthError::ProtocolError(error.to_string()))
        });

    body.zeroize();
    result
}

fn read_response(reader: &mut impl Read) -> Result<Response, AuthError> {
    let mut length_bytes = [0; 4];
    reader
        .read_exact(&mut length_bytes)
        .map_err(|error| AuthError::ProtocolError(error.to_string()))?;
    let length = usize::try_from(u32::from_ne_bytes(length_bytes))
        .map_err(|error| AuthError::ProtocolError(error.to_string()))?;

    if length > MAX_FRAME_BYTES {
        return Err(AuthError::ProtocolError(format!(
            "response exceeds {MAX_FRAME_BYTES} bytes"
        )));
    }

    let mut body = vec![0; length];
    let result = reader
        .read_exact(&mut body)
        .map_err(|error| AuthError::ProtocolError(error.to_string()))
        .and_then(|()| {
            serde_json::from_slice(&body)
                .map_err(|error| AuthError::ProtocolError(error.to_string()))
        });
    body.zeroize();
    result
}

impl AuthError {
    fn redact_secret(self, secret: &str) -> Self {
        fn redact(message: String, secret: &str) -> String {
            if secret.is_empty() {
                message
            } else {
                message.replace(secret, "[redacted]")
            }
        }

        match self {
            Self::ConnectionFailed(message) => Self::ConnectionFailed(redact(message, secret)),
            Self::ProtocolError(message) => Self::ProtocolError(redact(message, secret)),
            Self::AuthFailed => Self::AuthFailed,
            Self::RequestFailed(message) => Self::RequestFailed(redact(message, secret)),
            Self::InvalidSessionCommand(message) => {
                Self::InvalidSessionCommand(redact(message, secret))
            }
            Self::UnsupportedPrompt(message) => Self::UnsupportedPrompt(redact(message, secret)),
            Self::SessionCleanupFailed { cause, cleanup } => Self::SessionCleanupFailed {
                cause: Box::new(cause.redact_secret(secret)),
                cleanup: Box::new(cleanup.redact_secret(secret)),
            },
        }
    }
}

impl GreetdSession for GreetdClient {
    fn create_session(&mut self, username: &str) -> Result<AuthState, AuthError> {
        self.send(&Request::CreateSession {
            username: username.to_string(),
        })?;

        response_to_auth_state(self.receive()?)
    }

    fn post_auth_response(&mut self, response: Option<String>) -> Result<AuthState, AuthError> {
        let mut request = Request::PostAuthMessageResponse { response };
        let send_result = self.send(&request);
        if let Request::PostAuthMessageResponse {
            response: Some(secret),
        } = &mut request
        {
            secret.zeroize();
        }
        send_result?;
        response_to_auth_state(self.receive()?)
    }

    fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError> {
        self.send(&Request::StartSession { cmd, env: vec![] })?;

        match self.receive()? {
            Response::Success => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(response_error(&error_type, &description)),
            Response::AuthMessage { .. } => {
                Err(AuthError::ProtocolError("Unexpected response".into()))
            }
        }
    }

    fn cancel_session(&mut self) -> Result<(), AuthError> {
        self.send(&Request::CancelSession)?;

        match self.receive()? {
            Response::Success => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(response_error(&error_type, &description)),
            Response::AuthMessage { .. } => Err(AuthError::ProtocolError(
                "Unexpected response while cancelling session".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    NeedInput(String),
    NeedSecret(String),
    Info(String),
    Error(String),
    Done,
}

fn response_to_auth_state(response: Response) -> Result<AuthState, AuthError> {
    match response {
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
        } => Err(response_error(&error_type, &description)),
    }
}

fn response_error(error_type: &ErrorType, description: &str) -> AuthError {
    match error_type {
        ErrorType::AuthError => AuthError::AuthFailed,
        ErrorType::Error if description.is_empty() => {
            AuthError::RequestFailed("unspecified error".into())
        }
        ErrorType::Error => AuthError::RequestFailed(description.to_string()),
    }
}

/// Perform the supported greetd authentication flow and start the configured session.
pub fn authenticate(username: &str, password: &str, session_cmd: &str) -> Result<(), AuthError> {
    let mut client = GreetdClient::connect()?;
    authenticate_with_client(&mut client, username, password, session_cmd)
        .map_err(|error| error.redact_secret(password))
}

fn authenticate_with_client(
    client: &mut impl GreetdSession,
    username: &str,
    password: &str,
    session_cmd: &str,
) -> Result<(), AuthError> {
    let cmd = parse_session_command(session_cmd)?;
    let mut state = client.create_session(username)?;
    let mut password_sent = false;

    let result = (|| {
        loop {
            state = match state {
                AuthState::Done => break,
                AuthState::Info(_) | AuthState::Error(_) => client.post_auth_response(None)?,
                AuthState::NeedSecret(_) if !password_sent => {
                    password_sent = true;
                    client.post_auth_response(Some(password.to_string()))?
                }
                AuthState::NeedSecret(_) => {
                    return Err(AuthError::UnsupportedPrompt(
                        "additional secret prompt".into(),
                    ));
                }
                AuthState::NeedInput(_) => {
                    return Err(AuthError::UnsupportedPrompt("visible prompt".into()));
                }
            };
        }

        client.start_session(cmd)
    })();

    match result {
        Ok(()) => Ok(()),
        Err(cause) => match client.cancel_session() {
            Ok(()) => Err(cause),
            Err(cleanup) => Err(AuthError::SessionCleanupFailed {
                cause: Box::new(cause),
                cleanup: Box::new(cleanup),
            }),
        },
    }
}

fn parse_session_command(session_cmd: &str) -> Result<Vec<String>, AuthError> {
    let cmd = shell_words::split(session_cmd)
        .map_err(|error| AuthError::InvalidSessionCommand(error.to_string()))?;

    if cmd.is_empty() {
        return Err(AuthError::InvalidSessionCommand(
            "command cannot be empty".into(),
        ));
    }

    Ok(cmd)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests can unwrap")]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::thread;

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Create(String),
        Post(Option<String>),
        Start(Vec<String>),
        Cancel,
    }

    struct MockClient {
        states: VecDeque<AuthState>,
        calls: Vec<Call>,
        post_error: Option<AuthError>,
        cancel_error: Option<AuthError>,
    }

    impl MockClient {
        fn new(states: impl IntoIterator<Item = AuthState>) -> Self {
            Self {
                states: states.into_iter().collect(),
                calls: Vec::new(),
                post_error: None,
                cancel_error: None,
            }
        }

        fn with_post_error(mut self, error: AuthError) -> Self {
            self.post_error = Some(error);
            self
        }

        fn with_cancel_error(mut self, error: AuthError) -> Self {
            self.cancel_error = Some(error);
            self
        }

        fn next_state(&mut self) -> AuthState {
            self.states.pop_front().unwrap()
        }
    }

    impl GreetdSession for MockClient {
        fn create_session(&mut self, username: &str) -> Result<AuthState, AuthError> {
            self.calls.push(Call::Create(username.to_string()));
            Ok(self.next_state())
        }

        fn post_auth_response(&mut self, response: Option<String>) -> Result<AuthState, AuthError> {
            self.calls.push(Call::Post(response));
            if let Some(error) = self.post_error.take() {
                return Err(error);
            }
            Ok(self.next_state())
        }

        fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError> {
            self.calls.push(Call::Start(cmd));
            Ok(())
        }

        fn cancel_session(&mut self) -> Result<(), AuthError> {
            self.calls.push(Call::Cancel);
            self.cancel_error.take().map_or(Ok(()), Err)
        }
    }

    #[test]
    fn authenticates_secret_prompt_and_starts_session() {
        let mut client =
            MockClient::new([AuthState::NeedSecret("Password:".into()), AuthState::Done]);

        let result = authenticate_with_client(
            &mut client,
            "alice",
            "hunter2",
            "uwsm start hyprland.desktop",
        );

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(Some("hunter2".into())),
                Call::Start(vec![
                    "uwsm".into(),
                    "start".into(),
                    "hyprland.desktop".into()
                ]),
            ]
        );
    }

    #[test]
    fn handles_immediate_authentication_success() {
        let mut client = MockClient::new([AuthState::Done]);

        let result = authenticate_with_client(&mut client, "alice", "", "/bin/sh");

        assert!(result.is_ok());
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Start(vec!["/bin/sh".into()]),
            ]
        );
    }

    #[test]
    fn acknowledges_info_before_secret_prompt() {
        let mut client = MockClient::new([
            AuthState::Info("Welcome".into()),
            AuthState::NeedSecret("Password:".into()),
            AuthState::Done,
        ]);

        let result = authenticate_with_client(&mut client, "alice", "hunter2", "/bin/sh");

        assert!(result.is_ok());
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(None),
                Call::Post(Some("hunter2".into())),
                Call::Start(vec!["/bin/sh".into()]),
            ]
        );
    }

    #[test]
    fn cancels_unsupported_visible_prompt() {
        let mut client = MockClient::new([AuthState::NeedInput("One-time code:".into())]);

        let result = authenticate_with_client(&mut client, "alice", "hunter2", "/bin/sh");

        assert!(matches!(result, Err(AuthError::UnsupportedPrompt(_))));
        assert_eq!(client.calls, [Call::Create("alice".into()), Call::Cancel]);
    }

    #[test]
    fn cancels_additional_secret_prompt() {
        let mut client = MockClient::new([
            AuthState::NeedSecret("Password:".into()),
            AuthState::NeedSecret("One-time code:".into()),
        ]);

        let result = authenticate_with_client(&mut client, "alice", "hunter2", "/bin/sh");

        assert!(matches!(
            result,
            Err(AuthError::UnsupportedPrompt(message)) if message == "additional secret prompt"
        ));
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(Some("hunter2".into())),
                Call::Cancel,
            ]
        );
    }

    #[test]
    fn authentication_failure_stops_before_session_start() {
        let mut client = MockClient::new([AuthState::NeedSecret("Password:".into())])
            .with_post_error(AuthError::AuthFailed);

        let result = authenticate_with_client(&mut client, "alice", "test-secret", "/bin/sh");

        assert!(matches!(result, Err(AuthError::AuthFailed)));
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(Some("test-secret".into())),
                Call::Cancel,
            ]
        );
    }

    #[test]
    fn cancellation_failure_is_observable() {
        let mut client = MockClient::new([AuthState::NeedSecret("Password:".into())])
            .with_post_error(AuthError::AuthFailed)
            .with_cancel_error(AuthError::RequestFailed("cleanup denied".into()));

        let result = authenticate_with_client(&mut client, "alice", "test-secret", "/bin/sh");

        assert!(matches!(
            result,
            Err(AuthError::SessionCleanupFailed { cause, cleanup })
                if matches!(*cause, AuthError::AuthFailed)
                    && matches!(*cleanup, AuthError::RequestFailed(ref message)
                        if message == "cleanup denied")
        ));
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(Some("test-secret".into())),
                Call::Cancel,
            ]
        );
    }

    #[test]
    fn acknowledges_multiple_informational_messages_in_order() {
        let mut client = MockClient::new([
            AuthState::Info("Notice".into()),
            AuthState::Error("Previous failure".into()),
            AuthState::NeedSecret("Password:".into()),
            AuthState::Done,
        ]);

        let result = authenticate_with_client(&mut client, "alice", "hunter2", "/bin/sh");

        assert!(result.is_ok());
        assert_eq!(
            client.calls,
            [
                Call::Create("alice".into()),
                Call::Post(None),
                Call::Post(None),
                Call::Post(Some("hunter2".into())),
                Call::Start(vec!["/bin/sh".into()]),
            ]
        );
    }

    #[test]
    fn response_conversion_preserves_prompt_type() {
        let state = response_to_auth_state(Response::AuthMessage {
            auth_message_type: AuthMessageType::Secret,
            auth_message: "Password:".into(),
        })
        .unwrap();

        assert_eq!(state, AuthState::NeedSecret("Password:".into()));
        assert!(matches!(
            response_to_auth_state(Response::Error {
                error_type: ErrorType::AuthError,
                description: String::new(),
            }),
            Err(AuthError::AuthFailed)
        ));
    }

    #[test]
    fn rejects_invalid_or_empty_session_commands() {
        assert!(matches!(
            parse_session_command("unterminated '"),
            Err(AuthError::InvalidSessionCommand(_))
        ));
        assert!(matches!(
            parse_session_command("  "),
            Err(AuthError::InvalidSessionCommand(_))
        ));
    }

    #[test]
    fn formats_empty_auth_error_consistently() {
        assert_eq!(
            response_error(&ErrorType::AuthError, "").to_string(),
            "Authentication failed"
        );
        assert_eq!(
            response_error(&ErrorType::AuthError, "Access denied").to_string(),
            "Authentication failed"
        );
        assert_eq!(
            response_error(&ErrorType::Error, "").to_string(),
            "greetd request failed: unspecified error"
        );
    }

    #[test]
    fn authentication_errors_redact_the_submitted_secret() {
        let error =
            AuthError::RequestFailed("server echoed hunter2".into()).redact_secret("hunter2");

        assert!(matches!(
            error,
            AuthError::RequestFailed(message)
                if message == "server echoed [redacted]" && !message.contains("hunter2")
        ));

        assert_eq!(
            AuthError::AuthFailed.redact_secret("a").to_string(),
            "Authentication failed"
        );
    }

    #[test]
    fn cleanup_errors_redact_the_submitted_secret_recursively() {
        let error = AuthError::SessionCleanupFailed {
            cause: Box::new(AuthError::RequestFailed(
                "authentication echoed hunter2".into(),
            )),
            cleanup: Box::new(AuthError::ProtocolError("cleanup echoed hunter2".into())),
        }
        .redact_secret("hunter2")
        .to_string();

        assert!(!error.contains("hunter2"));
        assert_eq!(
            error,
            "greetd request failed: authentication echoed [redacted]; failed to cancel greetd session: Protocol error: cleanup echoed [redacted]"
        );
    }

    #[test]
    fn request_codec_matches_greetd_wire_format() {
        let mut framed = Vec::new();
        write_request(
            &mut framed,
            &Request::PostAuthMessageResponse {
                response: Some("test-secret".into()),
            },
        )
        .unwrap();

        let length = u32::from_ne_bytes(framed[0..4].try_into().unwrap()) as usize;
        assert_eq!(length, framed.len() - 4);
        let value: serde_json::Value = serde_json::from_slice(&framed[4..]).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "post_auth_message_response",
                "response": "test-secret"
            })
        );
    }

    #[test]
    fn request_codec_rejects_oversized_frame_before_writing() {
        let request = Request::StartSession {
            cmd: vec!["x".repeat(MAX_FRAME_BYTES)],
            env: vec![],
        };
        let mut framed = Vec::new();

        let result = write_request(&mut framed, &request);

        assert!(matches!(
            result,
            Err(AuthError::ProtocolError(message)) if message.contains("request exceeds")
        ));
        assert!(framed.is_empty());
    }

    #[test]
    fn response_codec_reads_greetd_wire_format() {
        let body =
            br#"{"type":"auth_message","auth_message_type":"secret","auth_message":"Password:"}"#;
        let mut framed = Vec::from(u32::try_from(body.len()).unwrap().to_ne_bytes());
        framed.extend_from_slice(body);

        let response = read_response(&mut Cursor::new(framed)).unwrap();

        assert!(matches!(
            response,
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                auth_message,
            } if auth_message == "Password:"
        ));
    }

    #[test]
    fn response_codec_rejects_oversized_frame_before_allocation() {
        let oversized = u32::try_from(MAX_FRAME_BYTES).unwrap() + 1;
        let result = read_response(&mut Cursor::new(oversized.to_ne_bytes()));

        assert!(matches!(
            result,
            Err(AuthError::ProtocolError(message)) if message.contains("response exceeds")
        ));
    }

    #[test]
    fn concrete_client_completes_fake_greetd_exchange() {
        let (client_stream, mut server_stream) = UnixStream::pair().unwrap();
        client_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let server = thread::spawn(move || {
            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({"type": "create_session", "username": "alice"})
            );
            write_response_value(
                &mut server_stream,
                &Response::AuthMessage {
                    auth_message_type: AuthMessageType::Secret,
                    auth_message: "Password:".into(),
                },
            );

            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({
                    "type": "post_auth_message_response",
                    "response": "test-secret"
                })
            );
            write_response_value(&mut server_stream, &Response::Success);

            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({
                    "type": "start_session",
                    "cmd": ["uwsm", "start", "hyprland.desktop"],
                    "env": []
                })
            );
            write_response_value(&mut server_stream, &Response::Success);
        });

        let mut client = GreetdClient {
            stream: client_stream,
        };
        let result = authenticate_with_client(
            &mut client,
            "alice",
            "test-secret",
            "uwsm start hyprland.desktop",
        );
        drop(client);

        server.join().unwrap();
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn concrete_client_cancels_failed_authentication() {
        let (client_stream, mut server_stream) = UnixStream::pair().unwrap();
        client_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        server_stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();

        let server = thread::spawn(move || {
            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({"type": "create_session", "username": "alice"})
            );
            write_response_value(
                &mut server_stream,
                &Response::AuthMessage {
                    auth_message_type: AuthMessageType::Secret,
                    auth_message: "Password:".into(),
                },
            );

            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({
                    "type": "post_auth_message_response",
                    "response": "test-secret"
                })
            );
            write_response_value(
                &mut server_stream,
                &Response::Error {
                    error_type: ErrorType::AuthError,
                    description: "authentication failed".into(),
                },
            );

            assert_eq!(
                read_request_value(&mut server_stream),
                serde_json::json!({"type": "cancel_session"})
            );
            write_response_value(&mut server_stream, &Response::Success);
        });

        let mut client = GreetdClient {
            stream: client_stream,
        };
        let result = authenticate_with_client(&mut client, "alice", "test-secret", "/bin/sh");
        drop(client);

        server.join().unwrap();
        assert!(matches!(result, Err(AuthError::AuthFailed)));
    }

    fn read_request_value(stream: &mut UnixStream) -> serde_json::Value {
        let mut length = [0; 4];
        stream.read_exact(&mut length).unwrap();
        let mut body = vec![0; u32::from_ne_bytes(length) as usize];
        stream.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn write_response_value(stream: &mut UnixStream, response: &Response) {
        let body = serde_json::to_vec(response).unwrap();
        let length = u32::try_from(body.len()).unwrap().to_ne_bytes();
        stream.write_all(&length).unwrap();
        stream.write_all(&body).unwrap();
    }
}
