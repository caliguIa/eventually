use std::{
    fs,
    io::{Error, ErrorKind, Result, Write},
    path::PathBuf,
    process::Command,
};

pub const ID: &str = "io.calrichards.eventually";

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub bin_path: PathBuf,
}

impl Service {
    pub fn try_new(name: &str) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            bin_path: std::env::current_exe()?,
        })
    }

    #[must_use]
    pub fn plist_path(&self) -> PathBuf {
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(format!("{}/Library/LaunchAgents/{}.plist", home, self.name))
    }

    #[must_use]
    pub fn log_path(&self, kind: &str) -> PathBuf {
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(format!("{}/Library/Logs/eventually.{}", home, kind))
    }

    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.plist_path().exists()
    }

    pub fn install(&self) -> Result<()> {
        let plist_path = self.plist_path();
        if self.is_installed() {
            eprintln!(
                "existing launch agent detected at `{}`, skipping installation",
                plist_path.display()
            );
            return Ok(());
        }

        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if let Some(parent) = self.log_path("log").parent() {
            fs::create_dir_all(parent)?;
        }

        let mut plist = fs::File::create(&plist_path)?;
        plist.write_all(self.launchd_plist().as_bytes())?;
        println!("installed launch agent to `{}`", plist_path.display());
        Ok(())
    }

    pub fn uninstall(&self) -> Result<()> {
        let plist_path = self.plist_path();
        if !self.is_installed() {
            eprintln!(
                "no launch agent detected at `{}`, skipping uninstallation",
                plist_path.display(),
            );
            return Ok(());
        }

        if let Err(e) = self.stop() {
            eprintln!("failed to stop service: {e:?}");
        }

        fs::remove_file(&plist_path)?;
        println!(
            "removed existing launch agent at `{}`",
            plist_path.display()
        );
        Ok(())
    }

    pub fn restart(&self) -> Result<()> {
        self.stop()?;
        self.start()
    }

    pub fn start(&self) -> Result<()> {
        if !self.is_installed() {
            self.install()?;
        }

        println!("starting service...");
        let output = Command::new("launchctl")
            .arg("load")
            .arg(self.plist_path())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already loaded") {
                println!("service already running");
                return Ok(());
            }
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to start service: {}", stderr),
            ));
        }

        println!("service started");
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        println!("stopping service...");
        let output = Command::new("launchctl")
            .arg("unload")
            .arg(self.plist_path())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Could not find") {
                println!("service not running");
                return Ok(());
            }
            return Err(Error::new(
                ErrorKind::Other,
                format!("Failed to stop service: {}", stderr),
            ));
        }

        println!("service stopped");
        Ok(())
    }

    #[must_use]
    pub fn launchd_plist(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}</string>
    <key>StandardErrorPath</key>
    <string>{}</string>
</dict>
</plist>
"#,
            self.name,
            self.bin_path.display(),
            self.log_path("log").display(),
            self.log_path("err").display(),
        )
    }
}
