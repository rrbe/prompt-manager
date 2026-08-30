mod add;
mod completions;
mod edit;
mod export;
mod favorite;
mod get;
mod history;
mod import;
mod list;
mod remove;
mod search;

use std::{
    env,
    io::{self, IsTerminal, Write},
    process::{Command as ProcessCommand, Stdio},
};

use time::{OffsetDateTime, UtcOffset};

use crate::{
    cli::{Command, CompletionsArgs},
    db::{Database, Prompt, PromptInput},
    error::{Error, Result},
    prompt::markdown::PromptDocument,
};

pub fn completions(arguments: CompletionsArgs) -> Result<()> {
    completions::run(arguments)
}

pub fn execute(command: Command, database: &mut Database) -> Result<()> {
    match command {
        Command::Add(arguments) => add::run(arguments, database),
        Command::Edit(arguments) => edit::run(arguments, database),
        Command::Rm(arguments) => remove::run(arguments, database),
        Command::Get(arguments) => get::run(arguments, database),
        Command::List(arguments) => list::run(arguments, database),
        Command::Search(arguments) => search::run(arguments, database),
        Command::Import(arguments) => import::run(arguments, database),
        Command::Export(arguments) => export::run(arguments, database),
        Command::Favorite(arguments) => favorite::run(arguments, database),
        Command::History(arguments) => history::run(arguments, database),
        Command::Completions(_) => {
            unreachable!("standalone command reached database dispatcher")
        }
    }
}

fn write_stdout(value: &str) -> Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    output.write_all(value.as_bytes())?;
    output.flush()?;
    Ok(())
}

fn write_paged_stdout(value: &str) -> Result<()> {
    if !io::stdout().is_terminal() {
        return write_stdout(value);
    }

    let command = pager_command()?;
    if !write_to_pager(value, &command)? {
        return write_stdout(value);
    }
    Ok(())
}

fn write_to_pager(value: &str, command: &[String]) -> Result<bool> {
    let (program, arguments) = command
        .split_first()
        .expect("pager command always contains a program");
    let mut pager = match ProcessCommand::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(pager) => pager,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    let write_result = pager
        .stdin
        .take()
        .expect("pager stdin is piped")
        .write_all(value.as_bytes());
    let status = pager.wait()?;

    if let Err(error) = write_result {
        if error.kind() == io::ErrorKind::BrokenPipe {
            return Ok(true);
        }
        return Err(error.into());
    }
    if !status.success() {
        return Err(Error::Message(format!("pager exited with status {status}")));
    }
    Ok(true)
}

fn pager_command() -> Result<Vec<String>> {
    let value = env::var("PAGER")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "less -FRX".into());
    let command = shell_words::split(&value)
        .map_err(|error| Error::Message(format!("invalid pager command: {error}")))?;
    if command.is_empty() {
        return Err(Error::Message("pager command must not be empty".into()));
    }
    Ok(command)
}

fn current_local_offset() -> Result<UtcOffset> {
    UtcOffset::current_local_offset()
        .map_err(|error| Error::Message(format!("failed to determine local time: {error}")))
}

fn format_local_timestamp(timestamp: i64, local_offset: UtcOffset) -> Result<String> {
    let timestamp = OffsetDateTime::from_unix_timestamp(timestamp)
        .map_err(|_| Error::Message(format!("timestamp is out of range: {timestamp}")))?
        .to_offset(local_offset);
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        timestamp.year(),
        timestamp.month() as u8,
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute()
    ))
}

fn format_table<const COLUMNS: usize>(
    headers: &[&str; COLUMNS],
    rows: &[[String; COLUMNS]],
) -> String {
    let widths = std::array::from_fn(|column| {
        rows.iter()
            .map(|row| row[column].chars().count())
            .chain([headers[column].chars().count()])
            .max()
            .unwrap_or_default()
    });

    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(format_table_row(headers, &widths));
    lines.push(
        widths
            .iter()
            .map(|width| "─".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    lines.extend(rows.iter().map(|row| format_table_row(row, &widths)));
    format!("{}\n", lines.join("\n"))
}

fn format_table_row<const COLUMNS: usize, S: AsRef<str>>(
    columns: &[S; COLUMNS],
    widths: &[usize; COLUMNS],
) -> String {
    columns
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let value = value.as_ref();
            let padding = widths[index] - value.chars().count();
            format!("{value}{}", " ".repeat(padding))
        })
        .collect::<Vec<_>>()
        .join("  ")
        .trim_end()
        .to_owned()
}

fn document_to_input(document: PromptDocument) -> PromptInput {
    PromptInput {
        name: document.name,
        description: document.description,
        content: document.content,
        tags: document.tags,
    }
}

fn prompt_to_document(prompt: Prompt) -> PromptDocument {
    PromptDocument {
        name: prompt.name,
        description: prompt.description,
        tags: prompt.tags,
        content: prompt.content,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn sends_output_to_pager_stdin() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("paged-output");
        let command = vec![
            "sh".to_owned(),
            "-c".to_owned(),
            "cat > \"$1\"".to_owned(),
            "pager".to_owned(),
            destination.to_string_lossy().into_owned(),
        ];

        assert!(write_to_pager("paged output\n", &command).unwrap());
        assert_eq!(fs::read_to_string(destination).unwrap(), "paged output\n");
    }
}
