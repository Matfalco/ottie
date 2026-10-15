//! Putting the computer to sleep, hibernating it or shutting it down, once
//! everything has finished (docs/decisions/0040): the system's own calls on
//! Windows, logind's (`systemctl`) on Linux. macOS comes later.

use serde::Serialize;

/// What the computer can be asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Sleep,
    Hibernate,
    ShutDown,
}

/// What this computer lets Ottie do, as the system tells it now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Able {
    pub sleep: bool,
    pub hibernate: bool,
    pub shut_down: bool,
}

impl Able {
    pub fn to(self, power: Power) -> bool {
        match power {
            Power::Sleep => self.sleep,
            Power::Hibernate => self.hibernate,
            Power::ShutDown => self.shut_down,
        }
    }
}

pub use system::{able, run};

#[cfg(windows)]
mod system {
    use std::io;
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID,
    };
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Power::{
        GetPwrCapabilities, SYSTEM_POWER_CAPABILITIES, SetSuspendState,
    };
    use windows_sys::Win32::System::Shutdown::{
        EWX_FORCEIFHUNG, EWX_POWEROFF, ExitWindowsEx, SHTDN_REASON_FLAG_PLANNED,
        SHTDN_REASON_MAJOR_APPLICATION,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    use super::{Able, Power};

    /// Sleep needs S1 to S3: a computer with modern standby (S0) has none,
    /// and no app can start its sleep. Hibernating needs S4 and the
    /// hibernation file (`powercfg /h off` removes it). Shutting down needs
    /// the privilege, which most accounts hold.
    pub fn able() -> Able {
        let mut capabilities = SYSTEM_POWER_CAPABILITIES::default();
        // SAFETY: the call fills the struct it's given, which lives here.
        let known = unsafe { GetPwrCapabilities(&mut capabilities) };
        let shut_down = may_shut_down().is_ok();
        if !known {
            // Unknown: offered, and a refusal is told when it comes.
            return Able {
                sleep: true,
                hibernate: true,
                shut_down,
            };
        }
        Able {
            sleep: capabilities.SystemS1 || capabilities.SystemS2 || capabilities.SystemS3,
            hibernate: capabilities.SystemS4 && capabilities.HiberFilePresent,
            shut_down,
        }
    }

    /// Asks for it; sleep and hibernation return once the computer wakes up,
    /// a shutdown as soon as it has begun.
    pub fn run(power: Power) -> io::Result<()> {
        match power {
            Power::Sleep | Power::Hibernate => {
                // SAFETY: plain values; nothing is borrowed.
                let done = unsafe { SetSuspendState(power == Power::Hibernate, false, false) };
                if done {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            }
            Power::ShutDown => {
                may_shut_down()?;
                // Programs may still save their work (no EWX_FORCE); one
                // that doesn't answer is closed.
                // SAFETY: plain values.
                let asked = unsafe {
                    ExitWindowsEx(
                        EWX_POWEROFF | EWX_FORCEIFHUNG,
                        SHTDN_REASON_MAJOR_APPLICATION | SHTDN_REASON_FLAG_PLANNED,
                    )
                };
                if asked == 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Switches on the shutdown privilege for this process (accounts hold it
    /// switched off); an error if the account doesn't hold it at all.
    fn may_shut_down() -> io::Result<()> {
        let mut token: HANDLE = ptr::null_mut();
        // SAFETY: the current process's pseudo-handle needs no closing; the
        // token is written into `token`.
        let opened = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
        };
        if opened == 0 {
            return Err(io::Error::last_os_error());
        }
        let result = enable_shutdown(token);
        // SAFETY: `token` was opened above and is closed once.
        unsafe { CloseHandle(token) };
        result
    }

    fn enable_shutdown(token: HANDLE) -> io::Result<()> {
        let mut luid = LUID::default();
        // SAFETY: the name is a constant wide string; the LUID lives here.
        if unsafe { LookupPrivilegeValueW(ptr::null(), SE_SHUTDOWN_NAME, &mut luid) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        // SAFETY: `token` is open with the rights asked; the new state lives
        // here, and the previous one isn't asked for.
        let adjusted = unsafe {
            AdjustTokenPrivileges(token, 0, &privileges, 0, ptr::null_mut(), ptr::null_mut())
        };
        // It succeeds even when the account lacks the privilege: only the
        // last error says so.
        // SAFETY: no arguments.
        let error = unsafe { GetLastError() };
        if adjusted == 0 {
            Err(io::Error::last_os_error())
        } else if error == ERROR_NOT_ALL_ASSIGNED {
            Err(io::Error::from_raw_os_error(error as i32))
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
mod system {
    use std::io;
    use std::process::{Command, Stdio};

    use super::{Able, Power};

    /// What logind says (`yes`, or `challenge`: it will ask for a password);
    /// without logind's answer, offered, and a refusal is told when it comes.
    pub fn able() -> Able {
        Able {
            sleep: can("CanSuspend"),
            hibernate: can("CanHibernate"),
            shut_down: can("CanPowerOff"),
        }
    }

    fn can(method: &str) -> bool {
        let answer = Command::new("busctl")
            .args([
                "call",
                "org.freedesktop.login1",
                "/org/freedesktop/login1",
                "org.freedesktop.login1.Manager",
                method,
            ])
            .stderr(Stdio::null())
            .output();
        match answer {
            // `s "yes"`
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                text.contains("\"yes\"") || text.contains("\"challenge\"")
            }
            _ => true,
        }
    }

    /// Asks logind; returns once it has taken the request.
    pub fn run(power: Power) -> io::Result<()> {
        let verb = match power {
            Power::Sleep => "suspend",
            Power::Hibernate => "hibernate",
            Power::ShutDown => "poweroff",
        };
        let status = Command::new("systemctl")
            .arg(verb)
            .stdin(Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("systemctl {verb}: {status}")))
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod system {
    use std::io;

    use super::{Able, Power};

    pub fn able() -> Able {
        Able {
            sleep: false,
            hibernate: false,
            shut_down: false,
        }
    }

    pub fn run(_: Power) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}
