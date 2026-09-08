#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

struct Installer {
    directory: TempDir,
    binary: String,
}

impl Installer {
    fn new(platform: &str) -> Self {
        let directory = TempDir::new().unwrap();
        let root = directory.path();
        let binary = format!("pm-v9.0.0-{platform}");
        fs::write(root.join(&binary), "#!/bin/sh\necho 'pm 9.0.0'\n").unwrap();
        let checksum = Command::new("shasum")
            .current_dir(root)
            .args(["-a", "256", &binary])
            .output()
            .unwrap();
        assert!(checksum.status.success());
        fs::write(root.join(format!("{binary}.sha256")), checksum.stdout).unwrap();

        fs::create_dir(root.join("bin")).unwrap();
        executable(
            &root.join("bin/uname"),
            "#!/bin/sh\ncase \"$1\" in -s) echo \"$TEST_OS\" ;; -m) echo \"$TEST_ARCH\" ;; esac\n",
        );
        executable(
            &root.join("bin/curl"),
            r#"#!/bin/sh
set -eu
if [ "${TEST_DOWNLOAD_FAIL:-}" = 1 ]; then exit 22; fi
if [ "$3" = /dev/null ]; then
  printf 'https://github.com/rrbe/prompt-manager/releases/tag/v9.0.0'
else
  cp "${TEST_RELEASE}/${4##*/}" "$3"
fi
"#,
        );
        Self { directory, binary }
    }

    fn command(&self, os: &str, arch: &str) -> Command {
        let root = self.directory.path();
        let mut command = Command::new("bash");
        command
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
            .env("PM_INSTALL_DIR", root.join("installed tools"))
            .env("TEST_RELEASE", root)
            .env("TEST_OS", os)
            .env("TEST_ARCH", arch)
            .env_remove("TEST_DOWNLOAD_FAIL")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    root.join("bin").display(),
                    std::env::var("PATH").unwrap()
                ),
            );
        command
    }
}

#[test]
fn installs_verified_release_for_each_platform() {
    for (os, arch, platform) in [
        ("Darwin", "arm64", "apple-arm"),
        ("Darwin", "x86_64", "apple-intel"),
        ("Linux", "aarch64", "linux-arm"),
        ("Linux", "x86_64", "linux-intel"),
    ] {
        let installer = Installer::new(platform);
        installer
            .command(os, arch)
            .assert()
            .success()
            .stdout("pm 9.0.0\n")
            .stderr(predicate::str::contains("OK"));
        Command::new(installer.directory.path().join("installed tools/pm"))
            .arg("--version")
            .assert()
            .success()
            .stdout("pm 9.0.0\n");
    }
}

#[test]
fn checksum_failure_preserves_existing_installation() {
    let installer = Installer::new("apple-arm");
    let root = installer.directory.path();
    fs::write(root.join(&installer.binary), "corrupt binary").unwrap();
    fs::create_dir(root.join("installed tools")).unwrap();
    let installed = root.join("installed tools/pm");
    fs::write(&installed, "existing binary").unwrap();

    installer.command("Darwin", "arm64").assert().failure();

    assert_eq!(fs::read_to_string(installed).unwrap(), "existing binary");
}

#[test]
fn download_failure_does_not_install() {
    let installer = Installer::new("apple-arm");
    installer
        .command("Darwin", "arm64")
        .env("TEST_DOWNLOAD_FAIL", "1")
        .assert()
        .failure();
    assert!(!installer.directory.path().join("installed tools").exists());
}

#[test]
fn unsupported_platform_does_not_install() {
    let installer = Installer::new("apple-arm");
    for (os, arch) in [("FreeBSD", "x86_64"), ("Linux", "armv7l")] {
        installer
            .command(os, arch)
            .assert()
            .failure()
            .stderr(predicate::str::contains("Unsupported"));
    }
    assert!(!installer.directory.path().join("installed tools").exists());
}
