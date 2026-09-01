use std::{
    env, fs,
    io::{self, IsTerminal, Write},
    path::Path,
    process::{Command, Stdio},
};

use tempfile::Builder;

use crate::error::{Error, Result};

pub fn edit_until_valid<T>(initial: &str, validate: impl Fn(&str) -> Result<T>) -> Result<T> {
    let mut file = Builder::new().suffix(".md").tempfile()?;
    file.write_all(initial.as_bytes())?;

    let editor = editor_command()?;
    let (program, arguments) = editor
        .split_first()
        .ok_or_else(|| Error::Message("editor command must not be empty".into()))?;

    edit_file_until_valid(
        file.path(),
        program,
        arguments,
        io::stdin().is_terminal(),
        validate,
        confirm_retry,
    )
}

fn edit_file_until_valid<T>(
    path: &Path,
    program: &str,
    arguments: &[String],
    retry_validation_errors: bool,
    validate: impl Fn(&str) -> Result<T>,
    mut retry: impl FnMut(&Error) -> Result<bool>,
) -> Result<T> {
    loop {
        let status = Command::new(program)
            .args(arguments)
            .arg(path)
            .stdin(editor_stdin())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(Error::EditorFailed(status));
        }

        let source = read_editor_file(path)?;
        match validate(&source) {
            Ok(value) => return Ok(value),
            Err(error) if retry_validation_errors => {
                if retry(&error)? {
                    continue;
                }
                return Err(Error::Message(
                    "edit aborted after validation failure".into(),
                ));
            }
            Err(error) => return Err(error),
        }
    }
}

fn editor_stdin() -> Stdio {
    #[cfg(unix)]
    if !io::stdin().is_terminal()
        && let Ok(terminal) = fs::File::open("/dev/tty")
    {
        return Stdio::from(terminal);
    }

    Stdio::inherit()
}

fn editor_command() -> Result<Vec<String>> {
    let value = env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".into());
    shell_words::split(&value)
        .map_err(|error| Error::Message(format!("invalid editor command: {error}")))
}

fn read_editor_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| Error::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    String::from_utf8(bytes).map_err(|_| Error::Message("edited prompt is not valid UTF-8".into()))
}

fn confirm_retry(validation_error: &Error) -> Result<bool> {
    let stdin = io::stdin();
    let stderr = io::stderr();
    let mut error_output = stderr.lock();
    writeln!(error_output, "validation failed: {validation_error}")?;

    loop {
        write!(error_output, "Reopen editor? [Y/n] ")?;
        error_output.flush()?;

        let mut response = String::new();
        if stdin.read_line(&mut response)? == 0 {
            return Ok(false);
        }
        match response.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(error_output, "Please answer y or n.")?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_editor_with_arguments() {
        let parsed = shell_words::split("code --wait").unwrap();
        assert_eq!(parsed, vec!["code", "--wait"]);
    }

    #[cfg(unix)]
    #[test]
    fn reopens_the_same_file_after_validation_failure() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let prompt_path = directory.path().join("prompt.md");
        let state_path = directory.path().join("state");
        let editor_path = directory.path().join("editor.sh");
        fs::write(&prompt_path, "initial").unwrap();
        fs::write(
            &editor_path,
            format!(
                "#!/bin/sh\nif [ -e '{}' ]; then\n  printf '%s' '---\nname: valid\n---\n\nbody' > \"$1\"\nelse\n  : > '{}'\n  printf '%s' '---\n---\n\ninvalid' > \"$1\"\nfi\n",
                state_path.display(),
                state_path.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&editor_path, fs::Permissions::from_mode(0o700)).unwrap();

        let mut retries = 0;
        let document = edit_file_until_valid(
            &prompt_path,
            editor_path.to_str().unwrap(),
            &[],
            true,
            crate::prompt::markdown::parse,
            |_| {
                retries += 1;
                Ok(true)
            },
        )
        .unwrap();

        assert_eq!(document.name, "valid");
        assert_eq!(retries, 1);
    }
}
