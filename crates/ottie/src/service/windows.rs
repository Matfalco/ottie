//! Windows: for you, a task in Task Scheduler that starts `ottie serve` when
//! you sign in, in your session; for the computer (`--system`), a Windows
//! service under its own virtual account, `NT SERVICE\Ottie`.

#![cfg_attr(not(windows), allow(dead_code))]

use super::launchd::escape;

/// The task's name in Task Scheduler, and the service's.
pub const NAME: &str = "Ottie";

/// The task, in Task Scheduler's XML. `user` is `DOMAIN\name`; `arguments`
/// one command line, already quoted.
pub fn task(user: &str, exe: &str, arguments: &str) -> String {
    let (user, exe, arguments) = (escape(user), escape(exe), escape(arguments));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<!-- Written by `ottie service install`; `ottie service uninstall` removes it. -->
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Ottie's downloads, without the app (ottie service install).</Description>
    <URI>\{NAME}</URI>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
    <RestartOnFailure>
      <Interval>PT1M</Interval>
      <Count>3</Count>
    </RestartOnFailure>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

/// Words as one command line, quoted the way programs split it again
/// (`CommandLineToArgvW`): backslashes count only before a quote.
pub fn command_line(words: &[String]) -> String {
    words
        .iter()
        .map(|word| quote(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote(word: &str) -> String {
    if !word.is_empty() && !word.contains([' ', '\t', '"']) {
        return word.to_owned();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for c in word.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            c => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(c);
                backslashes = 0;
            }
        }
    }
    // Before the closing quote, each one counts.
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(windows)]
pub use here::{detach_console, install, serve, status, uninstall};

#[cfg(windows)]
mod here {
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, PoisonError};
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result, bail};
    use tokio_util::sync::CancellationToken;
    use windows_service::service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
        ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
        ServiceInfo, ServiceSidType, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    use windows_service::{define_windows_service, service_dispatcher};

    use super::{NAME, command_line, task};
    use crate::serve::ServeArgs;
    use crate::service::{self, Install, answers, describe, still_answers, text, tool};

    /// The service's account: made by Windows for this service alone.
    const ACCOUNT: &str = r"NT SERVICE\Ottie";
    /// `ERROR_SERVICE_DOES_NOT_EXIST`.
    const NO_SERVICE: i32 = 1060;
    /// Stopping saves every running download first.
    const STOPPING: Duration = Duration::from_secs(30);

    /// `--detach-console`: the task starts `ottie.exe`, a console program,
    /// so Windows gives it a console window; letting go of it closes it.
    pub fn detach_console() {
        // SAFETY: FreeConsole takes nothing and only detaches this process
        // from its console; nothing has been written to it yet.
        unsafe {
            windows_sys::Win32::System::Console::FreeConsole();
        }
    }

    pub fn install(install: &Install) -> Result<()> {
        if install.system {
            install_service(install)
        } else {
            install_task(install)
        }
    }

    pub fn uninstall(system: bool) -> Result<()> {
        if system {
            uninstall_service()
        } else {
            uninstall_task()
        }
    }

    pub fn status() -> Result<Vec<String>> {
        let task = tool("schtasks", &["/Query", "/TN", NAME]).is_ok();
        // The task's own state is in the system's language: whether Ottie
        // answers says more (the last line).
        let mut lines = vec![describe(
            "For you",
            task.then_some("Task Scheduler, at sign-in"),
            None,
        )];
        let service = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .and_then(|manager| manager.open_service(NAME, ServiceAccess::QUERY_STATUS))
            .and_then(|service| service.query_status());
        lines.push(match service {
            Ok(status) => describe(
                "System-wide",
                Some("Windows service, at boot"),
                Some(status.current_state == ServiceState::Running),
            ),
            Err(_) => describe("System-wide", None, None),
        });
        Ok(lines)
    }

    // For you: a task at sign-in.

    fn install_task(install: &Install) -> Result<()> {
        if tool("schtasks", &["/Query", "/TN", NAME]).is_ok() {
            // Ends ours, if it runs: Task Scheduler ends it at once, and its
            // downloads carry on from their files.
            let _ = tool("schtasks", &["/End", "/TN", NAME]);
        }
        let log = &install.log;
        let mut args = install.args.clone();
        args.extend([
            "--detach-console".to_owned(),
            "--log-file".to_owned(),
            text(log)?,
        ]);
        let user = match (std::env::var("USERDOMAIN"), std::env::var("USERNAME")) {
            (Ok(domain), Ok(name)) => format!(r"{domain}\{name}"),
            _ => bail!("can't tell who you are (USERDOMAIN, USERNAME)"),
        };
        let xml = task(&user, &text(&install.exe)?, &command_line(&args));
        // Task Scheduler reads its XML files in UTF-16.
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
        let file = std::env::temp_dir().join("ottie-task.xml");
        std::fs::write(&file, bytes).with_context(|| format!("can't write {}", file.display()))?;
        let created = tool(
            "schtasks",
            &[
                OsStr::new("/Create"),
                OsStr::new("/TN"),
                OsStr::new(NAME),
                OsStr::new("/XML"),
                file.as_os_str(),
                OsStr::new("/F"),
            ],
        );
        let _ = std::fs::remove_file(&file);
        created?;
        // Ending it doesn't wait for it to be gone.
        let busy = still_answers(install.addr, Duration::from_secs(5));
        if !busy {
            tool("schtasks", &["/Run", "/TN", NAME])?;
        }
        println!("Ottie runs as a service for you: the task \"{NAME}\" in Task Scheduler.");
        println!("  It starts when you sign in. Log: {}", log.display());
        println!(
            "  It runs {}: after updating it, install again.",
            install.exe.display()
        );
        if busy {
            println!(
                "Another Ottie already answers at {}, so this one starts at your next sign-in \
                 (or once that one stops: schtasks /Run /TN {NAME}).",
                install.addr
            );
        }
        Ok(())
    }

    fn uninstall_task() -> Result<()> {
        if tool("schtasks", &["/Query", "/TN", NAME]).is_err() {
            println!("Ottie isn't installed as a service for you.");
            return Ok(());
        }
        let _ = tool("schtasks", &["/End", "/TN", NAME]);
        tool("schtasks", &["/Delete", "/TN", NAME, "/F"])?;
        println!("Ottie is no longer a service for you.");
        Ok(())
    }

    // For the computer: a Windows service.

    fn install_service(install: &Install) -> Result<()> {
        let manager =
            manager(ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)?;
        let access = ServiceAccess::QUERY_STATUS
            | ServiceAccess::START
            | ServiceAccess::STOP
            | ServiceAccess::CHANGE_CONFIG;
        let existing = manager.open_service(NAME, access).ok();
        if let Some(service) = &existing {
            stop(service)?;
        }
        // Not in the desktop app's folder: uninstalling the app would take
        // it away.
        let program_files =
            std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
        let exe = PathBuf::from(program_files)
            .join("Ottie Service")
            .join("ottie.exe");
        service::copy_exe(&install.exe, &exe)?;

        let log = &install.log;
        let folder = log.parent().context("the log has no folder")?;
        let mut args = install.args.clone();
        args.extend([
            "--windows-service".to_owned(),
            "--log-file".to_owned(),
            text(log)?,
        ]);
        let info = ServiceInfo {
            name: NAME.into(),
            display_name: NAME.into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe.clone(),
            launch_arguments: args.into_iter().map(OsString::from).collect(),
            dependencies: Vec::new(),
            account_name: Some(ACCOUNT.into()),
            account_password: None,
        };
        let service = match existing {
            Some(service) => {
                service.change_config(&info)?;
                service
            }
            None => manager.create_service(&info, access)?,
        };
        // Its own SID, which `NT SERVICE\Ottie` names (for the folder's rights).
        service.set_config_service_sid_info(ServiceSidType::Unrestricted)?;
        service.set_description(
            "Ottie's downloads, for everyone on this computer (ottie service install --system).",
        )?;
        // Downloads can wait for the rest of Windows to start.
        service.set_delayed_auto_start(true)?;
        let restart = ServiceAction {
            action_type: ServiceActionType::Restart,
            delay: Duration::from_secs(5),
        };
        service.update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 60 * 60)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![restart.clone(), restart.clone(), restart]),
        })?;
        // Also when it stops with an error, not only when it crashes.
        service.set_failure_actions_on_non_crash_failures(true)?;
        service.set_preshutdown_timeout(STOPPING)?;

        // Only after the service exists: its account comes with it.
        private_folder(folder)?;
        let downloads = folder.join("Downloads");
        std::fs::create_dir_all(&downloads)
            .with_context(|| format!("can't create {}", downloads.display()))?;
        // Everyone on the computer can read what was downloaded there.
        icacls(&downloads, &["/grant", "*S-1-5-32-545:(OI)(CI)RX"])?;

        let busy = answers(install.addr);
        if !busy {
            service.start(&[] as &[&OsStr])?;
        }
        println!("Ottie runs as a Windows service for this computer, as {ACCOUNT}.");
        println!("  It starts at boot. Log: {}", log.display());
        println!(
            "  Its folder, for administrators only: {} (config.toml, token, database).",
            folder.display()
        );
        println!(
            "  Downloads go to {} until you choose a folder, which {ACCOUNT} must be able to write to.",
            downloads.display()
        );
        println!("  Its address, signed in: ottie link --system (as an administrator)");
        println!("  It runs a copy of ottie: {}", exe.display());
        if busy {
            println!(
                "Another Ottie already answers at {}, so this one starts at the next boot \
                 (or once that one stops: sc start {NAME}).",
                install.addr
            );
        }
        Ok(())
    }

    fn uninstall_service() -> Result<()> {
        let manager = manager(ServiceManagerAccess::CONNECT)?;
        let access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        let service = match manager.open_service(NAME, access) {
            Ok(service) => service,
            Err(windows_service::Error::Winapi(err)) if err.raw_os_error() == Some(NO_SERVICE) => {
                println!("Ottie isn't installed as a service for this computer.");
                return Ok(());
            }
            Err(err) => return Err(err).context("can't open the Ottie service"),
        };
        stop(&service)?;
        service.delete()?;
        println!("Ottie is no longer a service for this computer.");
        let folder = crate::config::system_folder()?;
        println!(
            "  Kept: {} (downloads, database, token) and its copy of ottie in Program Files\\Ottie Service.",
            folder.display()
        );
        Ok(())
    }

    fn manager(access: ServiceManagerAccess) -> Result<ServiceManager> {
        ServiceManager::local_computer(None::<&str>, access)
            .context("can't reach the Service Control Manager: run this from a terminal opened as administrator")
    }

    /// Asks it to stop (it saves its downloads) and waits until it has.
    fn stop(service: &windows_service::service::Service) -> Result<()> {
        if service.query_status()?.current_state == ServiceState::Stopped {
            return Ok(());
        }
        // Already stopping is fine too.
        let _ = service.stop();
        let start = Instant::now();
        while service.query_status()?.current_state != ServiceState::Stopped {
            if start.elapsed() > STOPPING {
                bail!(
                    "the Ottie service didn't stop within {} s",
                    STOPPING.as_secs()
                );
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Ok(())
    }

    /// Only the system, administrators and the service: the token is there.
    fn private_folder(folder: &Path) -> Result<()> {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("can't create {}", folder.display()))?;
        icacls(
            folder,
            &[
                "/inheritance:r",
                "/grant:r",
                // SYSTEM, Administrators: by their SIDs, whatever the
                // system's language calls them.
                "*S-1-5-18:(OI)(CI)F",
                "*S-1-5-32-544:(OI)(CI)F",
                &format!("{ACCOUNT}:(OI)(CI)M"),
            ],
        )
    }

    fn icacls(path: &Path, args: &[&str]) -> Result<()> {
        let mut all = vec![path.as_os_str()];
        all.extend(args.iter().map(OsStr::new));
        tool("icacls", &all)
            .map(drop)
            .with_context(|| format!("can't set who may use {}", path.display()))
    }

    // Running as the service.

    /// What `serve` hands the service's thread.
    static ARGS: Mutex<Option<ServeArgs>> = Mutex::new(None);

    define_windows_service!(ffi_service_main, service_main);

    /// `ottie serve --windows-service`: hands this thread to the Service
    /// Control Manager, which runs `service_main` on another one.
    pub fn serve(args: ServeArgs) -> Result<()> {
        *ARGS.lock().unwrap_or_else(PoisonError::into_inner) = Some(args);
        service_dispatcher::start(NAME, ffi_service_main)
            .context("only Windows' Service Control Manager starts `ottie serve --windows-service`")
    }

    fn service_main(_: Vec<OsString>) {
        let Some(args) = ARGS.lock().unwrap_or_else(PoisonError::into_inner).take() else {
            return;
        };
        let stop = CancellationToken::new();
        let stop_asked = stop.clone();
        let handler = move |control| match control {
            ServiceControl::Stop | ServiceControl::Preshutdown => {
                stop_asked.cancel();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let Ok(status) = service_control_handler::register(NAME, handler) else {
            return;
        };
        let report = |state, accepts, exit_code| {
            let _ = status.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted: accepts,
                exit_code,
                checkpoint: 0,
                wait_hint: Duration::ZERO,
                process_id: None,
            });
        };
        report(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::PRESHUTDOWN,
            ServiceExitCode::Win32(0),
        );
        let result = crate::block_on(crate::serve::run(args, stop));
        // An error is in the log; a code that isn't 0 makes Windows restart it.
        let code = match result {
            Ok(()) => ServiceExitCode::Win32(0),
            Err(_) => ServiceExitCode::ServiceSpecific(1),
        };
        report(ServiceState::Stopped, ServiceControlAccept::empty(), code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_quoted_for_windows() {
        let words = |list: &[&str]| list.iter().map(|&w| w.to_owned()).collect::<Vec<_>>();
        assert_eq!(command_line(&words(&["serve", "--x"])), "serve --x");
        assert_eq!(
            command_line(&words(&[r"C:\Users\Ann Lee\AppData\Local\Ottie\ottie.log"])),
            r#""C:\Users\Ann Lee\AppData\Local\Ottie\ottie.log""#
        );
        // Backslashes before a quote, or before the closing one, double.
        assert_eq!(command_line(&words(&[r"C:\a b\"])), r#""C:\a b\\""#);
        assert_eq!(command_line(&words(&[r#"say "hi""#])), r#""say \"hi\"""#);
        assert_eq!(command_line(&words(&[""])), r#""""#);
    }

    #[test]
    fn the_task_starts_ottie_at_sign_in_without_a_time_limit() {
        let xml = task(
            r"PC\Ann & Bo",
            r"C:\Users\Ann\.cargo\bin\ottie.exe",
            r#"serve --log-file "C:\l\ottie.log""#,
        );
        assert!(xml.contains(r"<UserId>PC\Ann &amp; Bo</UserId>"));
        assert!(xml.contains(r"<Command>C:\Users\Ann\.cargo\bin\ottie.exe</Command>"));
        assert!(
            xml.contains(r"<Arguments>serve --log-file &quot;C:\l\ottie.log&quot;</Arguments>")
        );
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
    }
}
