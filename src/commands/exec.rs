use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    cli::ExecArgs,
    db::Database,
    error::{Error, Result},
    prompt::parse_exec_command,
};

use super::get;

pub fn run(arguments: ExecArgs, database: &mut Database) -> Result<()> {
    let rendered = get::render(arguments.prompt, database)?;
    let command = rendered
        .exec
        .as_deref()
        .ok_or_else(|| Error::Message(format!("prompt has no exec command: {}", rendered.name)))?;
    let configured_arguments = parse_exec_command(command)?;
    let (program, configured_arguments) = configured_arguments
        .split_first()
        .expect("validated exec command contains a program");

    let mut child = Command::new(program)
        .args(configured_arguments)
        .args(arguments.arguments)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|error| {
            Error::Message(format!("failed to start exec command `{program}`: {error}"))
        })?;

    let write_result = child
        .stdin
        .take()
        .expect("exec command stdin is piped")
        .write_all(rendered.content.as_bytes());
    let usage_result = database.mark_prompt_used(&rendered.name);
    let status = child.wait()?;

    if !status.success() {
        return Err(Error::ExecFailed(status));
    }
    if let Err(error) = write_result {
        return Err(Error::Message(format!(
            "failed to write prompt to exec command: {error}"
        )));
    }
    usage_result
}
