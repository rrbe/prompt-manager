mod add;
mod check;
mod completions;
mod diff;
mod edit;
mod export;
mod favorite;
mod get;
mod history;
mod import;
mod list;
mod pick;
mod recent;
mod remove;
mod render;
mod search;

use std::io::{self, Write};

use crate::{
    cli::{CheckArgs, Command, CompletionsArgs},
    db::{Database, Prompt, PromptInput},
    error::Result,
    prompt::markdown::PromptDocument,
};

pub fn check(arguments: CheckArgs) -> Result<()> {
    check::run(arguments)
}

pub fn completions(arguments: CompletionsArgs) -> Result<()> {
    completions::run(arguments)
}

pub fn execute(command: Command, database: &mut Database) -> Result<()> {
    match command {
        Command::Add(arguments) => add::run(arguments, database),
        Command::Edit(arguments) => edit::run(arguments, database),
        Command::Rm(arguments) => remove::run(arguments, database),
        Command::Get(arguments) => get::run(arguments, database),
        Command::Render(arguments) => render::run(arguments, database),
        Command::List(arguments) => list::run(arguments, database),
        Command::Search(arguments) => search::run(arguments, database),
        Command::Recent => recent::run(database),
        Command::Pick => pick::run(database),
        Command::Import(arguments) => import::run(arguments, database),
        Command::Export(arguments) => export::run(arguments, database),
        Command::Favorite(arguments) => favorite::run(arguments, database),
        Command::History(arguments) => history::run(arguments, database),
        Command::Diff(arguments) => diff::run(arguments, database),
        Command::Check(_) | Command::Completions(_) => {
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
