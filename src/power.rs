//! Power management commands (shutdown, reboot, suspend)

use std::process::Command;

pub fn shutdown() {
    Command::new("systemctl").arg("poweroff").spawn().ok();
}

pub fn reboot() {
    Command::new("systemctl").arg("reboot").spawn().ok();
}

pub fn suspend() {
    Command::new("systemctl").arg("suspend").spawn().ok();
}
