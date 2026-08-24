use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    db::Database,
    error::{Error, Result},
};

use super::write_stdout;

pub fn run(database: &Database) -> Result<()> {
    let names = database.list_prompt_names()?;
    if names.is_empty() {
        return Err(Error::Message("no prompts available to pick".into()));
    }

    let mut child = Command::new("fzf")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::Message("fzf is not installed or not in PATH".into())
            } else {
                Error::Io(error)
            }
        })?;

    if let Some(mut input) = child.stdin.take() {
        let candidates = format!("{}\n", names.join("\n"));
        if let Err(error) = input.write_all(candidates.as_bytes())
            && error.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(error.into());
        }
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Message("fzf exited without a selection".into()));
    }
    let selection = String::from_utf8(output.stdout)
        .map_err(|_| Error::Message("fzf returned a non-UTF-8 selection".into()))?;
    let selection = selection.trim_end_matches(['\r', '\n']);
    if selection.is_empty() || !names.iter().any(|name| name == selection) {
        return Err(Error::Message("fzf returned an invalid selection".into()));
    }

    write_stdout(&format!("{selection}\n"))
}
