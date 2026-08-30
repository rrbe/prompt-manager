use std::io::{self, Write};

use self_update::backends::github;

use crate::{
    cli::UpdateArgs,
    error::{Error, Result},
};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_OWNER: &str = "rrbe";
const REPO_NAME: &str = "prompt-manager";

pub fn run(arguments: UpdateArgs) -> Result<()> {
    let stderr = io::stderr();
    run_with(arguments, &GitHubUpdater, &mut stderr.lock())
}

trait Updater {
    fn available_version(&self) -> Result<Option<String>>;
    fn install(&self, version: &str) -> Result<String>;
}

struct GitHubUpdater;

impl Updater for GitHubUpdater {
    fn available_version(&self) -> Result<Option<String>> {
        let updater = base_builder()
            .build()
            .map_err(|error| update_error("configure update", error))?;
        let releases = updater
            .get_latest_release()
            .map_err(|error| update_error("check for updates", error))?;
        let available = releases
            .is_update_available()
            .map_err(|error| update_error("compare versions", error))?;
        Ok(available.then(|| {
            releases
                .latest()
                .expect("latest release response contains one release")
                .version()
                .to_owned()
        }))
    }

    fn install(&self, version: &str) -> Result<String> {
        let target = self_update::get_target();
        let asset_name = format!("pm-v{version}-{target}.tar.gz");
        let checksum_name = format!("{asset_name}.sha256");
        let expected_asset_name = asset_name.clone();
        let mut builder = base_builder();
        builder
            .release_tag(format!("v{version}"))
            .asset_matcher(move |assets| {
                assets
                    .iter()
                    .find(|asset| asset.name() == expected_asset_name)
                    .cloned()
            })
            .bin_path_in_archive("pm-v{{ version }}-{{ target }}/{{ bin }}")
            .checksum_from_asset(checksum_name)
            .check_install_path_writable(true)
            .unattended();
        let status = builder
            .build()
            .map_err(|error| update_error("configure update", error))?
            .update()
            .map_err(|error| update_error("install update", error))?;
        Ok(status.version().to_owned())
    }
}

fn base_builder() -> github::UpdateBuilder {
    let mut builder = github::Update::configure();
    builder
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("pm")
        .target(self_update::get_target())
        .current_version(CURRENT_VERSION)
        .auth_token_from_env();
    builder
}

fn update_error(action: &str, error: self_update::Error) -> Error {
    Error::Message(format!("failed to {action}: {error}"))
}

fn run_with(arguments: UpdateArgs, updater: &impl Updater, output: &mut impl Write) -> Result<()> {
    let Some(version) = updater.available_version()? else {
        writeln!(output, "pm {CURRENT_VERSION} is up to date")?;
        return Ok(());
    };

    if arguments.check {
        writeln!(
            output,
            "pm {version} is available (current: {CURRENT_VERSION})"
        )?;
        return Ok(());
    }

    let installed_version = updater.install(&version)?;
    writeln!(
        output,
        "updated pm from {CURRENT_VERSION} to {installed_version}"
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct FakeUpdater {
        available: Option<String>,
        installed: Cell<bool>,
    }

    impl Updater for FakeUpdater {
        fn available_version(&self) -> Result<Option<String>> {
            Ok(self.available.clone())
        }

        fn install(&self, version: &str) -> Result<String> {
            self.installed.set(true);
            Ok(version.to_owned())
        }
    }

    #[test]
    fn check_reports_available_version_without_installing() {
        let updater = FakeUpdater {
            available: Some("9.0.0".into()),
            installed: Cell::new(false),
        };
        let mut output = Vec::new();

        run_with(UpdateArgs { check: true }, &updater, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("pm 9.0.0 is available (current: {CURRENT_VERSION})\n")
        );
        assert!(!updater.installed.get());
    }

    #[test]
    fn update_installs_available_version() {
        let updater = FakeUpdater {
            available: Some("9.0.0".into()),
            installed: Cell::new(false),
        };
        let mut output = Vec::new();

        run_with(UpdateArgs { check: false }, &updater, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("updated pm from {CURRENT_VERSION} to 9.0.0\n")
        );
        assert!(updater.installed.get());
    }

    #[test]
    fn reports_when_current_version_is_latest() {
        let updater = FakeUpdater {
            available: None,
            installed: Cell::new(false),
        };
        let mut output = Vec::new();

        run_with(UpdateArgs { check: false }, &updater, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("pm {CURRENT_VERSION} is up to date\n")
        );
        assert!(!updater.installed.get());
    }
}
