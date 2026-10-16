//! macOS: a LaunchAgent in `~/Library/LaunchAgents`, started at login.
//! (No system-wide one yet: it would need an account of its own.)

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

pub const LABEL: &str = "io.github.matfalco.ottie.service";

/// The property list: what to run, at load (login), again after a crash.
pub fn plist(exe: &str, args: &[String]) -> String {
    let words = std::iter::once(exe)
        .chain(args.iter().map(String::as_str))
        .map(|word| format!("\t\t<string>{}</string>\n", escape(word)))
        .collect::<String>();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <!-- Written by `ottie service install`; `ottie service uninstall` removes it. -->\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{LABEL}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         {words}\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         \t<key>KeepAlive</key>\n\
         \t<dict>\n\
         \t\t<key>SuccessfulExit</key>\n\
         \t\t<false/>\n\
         \t</dict>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// Text in XML.
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(target_os = "macos")]
pub use here::{install, status, uninstall};

#[cfg(target_os = "macos")]
mod here {
    use std::path::PathBuf;
    use std::process::Command;

    use anyhow::{Context, Result, bail};
    use directories::BaseDirs;

    use super::{LABEL, plist};
    use crate::service::{Install, answers, describe, text, tool};

    pub fn install(install: &Install) -> Result<()> {
        if install.system {
            bail!("Ottie has no system-wide service on macOS yet");
        }
        let domain = domain()?;
        if loaded(&domain) {
            // It stops as with Ctrl+C: downloads are saved.
            tool("launchctl", &["bootout", &format!("{domain}/{LABEL}")])?;
        }
        let log = &install.log;
        let mut args = install.args.clone();
        args.extend(["--log-file".to_owned(), text(log)?]);
        let path = path()?;
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)
                .with_context(|| format!("can't create {}", folder.display()))?;
        }
        std::fs::write(&path, plist(&text(&install.exe)?, &args))
            .with_context(|| format!("can't write {}", path.display()))?;
        let busy = answers(install.addr);
        if !busy {
            tool("launchctl", &["bootstrap", &domain, &text(&path)?])?;
        }
        println!("Ottie runs as a service for you.");
        println!("  LaunchAgent: {}", path.display());
        println!("  It starts when you log in. Log: {}", log.display());
        println!(
            "  It runs {}: after updating it, install again.",
            install.exe.display()
        );
        if busy {
            println!(
                "Another Ottie already answers at {}, so this one starts at your next login.",
                install.addr
            );
        }
        Ok(())
    }

    pub fn uninstall(system: bool) -> Result<()> {
        if system {
            bail!("Ottie has no system-wide service on macOS");
        }
        let path = path()?;
        if !path.exists() {
            println!("Ottie isn't installed as a service for you.");
            return Ok(());
        }
        let domain = domain()?;
        if loaded(&domain) {
            tool("launchctl", &["bootout", &format!("{domain}/{LABEL}")])?;
        }
        std::fs::remove_file(&path).with_context(|| format!("can't remove {}", path.display()))?;
        println!("Ottie is no longer a service for you.");
        Ok(())
    }

    pub fn status() -> Result<Vec<String>> {
        let installed = path()?.exists();
        let running = installed.then(|| {
            domain().is_ok_and(|domain| {
                Command::new("launchctl")
                    .args(["print", &format!("{domain}/{LABEL}")])
                    .output()
                    .is_ok_and(|output| {
                        String::from_utf8_lossy(&output.stdout).contains("state = running")
                    })
            })
        });
        Ok(vec![
            describe(
                "For you",
                installed.then_some("LaunchAgent, at login"),
                running,
            ),
            "System-wide: not on macOS yet".to_owned(),
        ])
    }

    fn path() -> Result<PathBuf> {
        let base = BaseDirs::new().context("can't find your home folder")?;
        Ok(base
            .home_dir()
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{LABEL}.plist")))
    }

    /// Your login session's services.
    fn domain() -> Result<String> {
        Ok(format!("gui/{}", tool("id", &["-u"])?))
    }

    fn loaded(domain: &str) -> bool {
        Command::new("launchctl")
            .args(["print", &format!("{domain}/{LABEL}")])
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plist_runs_ottie_and_keeps_it_alive() {
        let plist = plist(
            "/Users/ann & bo/ottie",
            &["serve".into(), "--log-file".into(), "/l/ottie.log".into()],
        );
        assert!(plist.contains("<string>/Users/ann &amp; bo/ottie</string>"));
        assert!(plist.contains("\t\t<string>serve</string>\n\t\t<string>--log-file</string>"));
        assert!(plist.contains(&format!("<string>{LABEL}</string>")));
        assert!(plist.contains("<key>SuccessfulExit</key>\n\t\t<false/>"));
    }
}
