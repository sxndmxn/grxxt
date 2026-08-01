//! greetd IPC communication module

use greetd_ipc::codec::SyncCodec;
use greetd_ipc::{AuthMessageType, ErrorType, Request, Response};
use std::env;
use std::os::unix::net::UnixStream;

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
    #[error("{0}")]
    AuthFailed(String),
    #[error("Invalid session command: {0}")]
    InvalidSessionCommand(String),
    #[error("Unsupported authentication prompt: {0}")]
    UnsupportedPrompt(String),
}

impl GreetdClient {
    pub fn connect() -> Result<Self, AuthError> {
        let socket_path = env::var("GREETD_SOCK")
            .map_err(|_| AuthError::ConnectionFailed("GREETD_SOCK not set".into()))?;

        let stream = UnixStream::connect(&socket_path)
            .map_err(|error| AuthError::ConnectionFailed(error.to_string()))?;

        Ok(Self { stream })
    }

    fn send(&mut self, request: &Request) -> Result<(), AuthError> {
        request
            .write_to(&mut self.stream)
            .map_err(|error| AuthError::ProtocolError(error.to_string()))
    }

    fn receive(&mut self) -> Result<Response, AuthError> {
        Response::read_from(&mut self.stream)
            .map_err(|error| AuthError::ProtocolError(error.to_string()))
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
        self.send(&Request::PostAuthMessageResponse { response })?;
        response_to_auth_state(self.receive()?)
    }

    fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError> {
        self.send(&Request::StartSession { cmd, env: vec![] })?;

        match self.receive()? {
            Response::Success => Ok(()),
            Response::Error {
                error_type,
                description,
            } => Err(AuthError::AuthFailed(format_error(
                &error_type,
                &description,
            ))),
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
            } => Err(AuthError::AuthFailed(format_error(
                &error_type,
                &description,
            ))),
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
        } => Err(AuthError::AuthFailed(format_error(
            &error_type,
            &description,
        ))),
    }
}

fn format_error(error_type: &ErrorType, description: &str) -> String {
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

/// Perform the supported greetd authentication flow and start the configured session.
pub fn authenticate(username: &str, password: &str, session_cmd: &str) -> Result<(), AuthError> {
    let mut client = GreetdClient::connect()?;
    authenticate_with_client(&mut client, username, password, session_cmd)
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

    loop {
        state = match state {
            AuthState::Done => break,
            AuthState::Info(_) | AuthState::Error(_) => client.post_auth_response(None)?,
            AuthState::NeedSecret(_) if !password_sent => {
                password_sent = true;
                client.post_auth_response(Some(password.to_string()))?
            }
            AuthState::NeedSecret(message) => {
                client.cancel_session().ok();
                return Err(AuthError::UnsupportedPrompt(format!(
                    "additional secret prompt: {message}"
                )));
            }
            AuthState::NeedInput(message) => {
                client.cancel_session().ok();
                return Err(AuthError::UnsupportedPrompt(format!(
                    "visible prompt: {message}"
                )));
            }
        };
    }

    client.start_session(cmd)
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
    }

    impl MockClient {
        fn new(states: impl IntoIterator<Item = AuthState>) -> Self {
            Self {
                states: states.into_iter().collect(),
                calls: Vec::new(),
            }
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
            Ok(self.next_state())
        }

        fn start_session(&mut self, cmd: Vec<String>) -> Result<(), AuthError> {
            self.calls.push(Call::Start(cmd));
            Ok(())
        }

        fn cancel_session(&mut self) -> Result<(), AuthError> {
            self.calls.push(Call::Cancel);
            Ok(())
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
            Err(AuthError::AuthFailed(message)) if message == "Authentication failed"
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
            format_error(&ErrorType::AuthError, ""),
            "Authentication failed"
        );
        assert_eq!(
            format_error(&ErrorType::AuthError, "Access denied"),
            "Access denied"
        );
    }
}
