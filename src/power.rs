//! Power management commands (shutdown, reboot, suspend)

use std::io;
use std::process::{Command, ExitStatus, Stdio};

const SYSTEMCTL_PATH: &str = "/usr/bin/systemctl";

/// Power operation requested by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerAction {
    PowerOff,
    Reboot,
    Suspend,
}

impl PowerAction {
    const fn systemctl_verb(self) -> &'static str {
        match self {
            Self::PowerOff => "poweroff",
            Self::Reboot => "reboot",
            Self::Suspend => "suspend",
        }
    }

    const fn systemctl_args(self) -> [&'static str; 3] {
        ["--no-block", "--no-ask-password", self.systemctl_verb()]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PowerError {
    #[error("failed to run systemctl {action}: {source}")]
    Start {
        action: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("systemctl {action} failed with {status}")]
    Failed {
        action: &'static str,
        status: ExitStatus,
    },
}

/// Run a power operation and wait for `systemctl` so no child is left unreaped.
pub fn execute(action: PowerAction) -> Result<(), PowerError> {
    let verb = action.systemctl_verb();
    let status = Command::new(SYSTEMCTL_PATH)
        .args(action.systemctl_args())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|source| PowerError::Start {
            action: verb,
            source,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(PowerError::Failed {
            action: verb,
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_map_to_fixed_systemctl_verbs() {
        assert_eq!(
            PowerAction::PowerOff.systemctl_args(),
            ["--no-block", "--no-ask-password", "poweroff"]
        );
        assert_eq!(
            PowerAction::Reboot.systemctl_args(),
            ["--no-block", "--no-ask-password", "reboot"]
        );
        assert_eq!(
            PowerAction::Suspend.systemctl_args(),
            ["--no-block", "--no-ask-password", "suspend"]
        );
    }
}
