use std::process::Command;

use clap::CommandFactory;
use clap_complete::generate;

use crate::{
    cli::{Cli, CompletionShell, CompletionsArgs},
    error::{Error, Result},
};

use super::write_stdout;

pub fn run(arguments: CompletionsArgs) -> Result<()> {
    if arguments.dynamic {
        return dynamic(arguments.shell);
    }

    let mut command = Cli::command();
    let mut output = Vec::new();
    generate(shell(arguments.shell), &mut command, "pm", &mut output);
    let output = String::from_utf8(output)
        .map_err(|_| Error::Message("generated completion is not valid UTF-8".into()))?;
    write_stdout(&output)
}

fn dynamic(shell: CompletionShell) -> Result<()> {
    let executable = std::env::current_exe()?;
    let output = Command::new(executable)
        .env("COMPLETE", shell.as_str())
        .output()?;
    if !output.status.success() {
        return Err(Error::Message(
            "failed to generate dynamic completion script".into(),
        ));
    }
    let output = String::from_utf8(output.stdout)
        .map_err(|_| Error::Message("generated completion is not valid UTF-8".into()))?;
    write_stdout(&output)
}

fn shell(shell: CompletionShell) -> clap_complete::Shell {
    match shell {
        CompletionShell::Bash => clap_complete::Shell::Bash,
        CompletionShell::Zsh => clap_complete::Shell::Zsh,
        CompletionShell::Fish => clap_complete::Shell::Fish,
    }
}
