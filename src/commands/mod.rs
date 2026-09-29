mod add;
mod completions;
mod edit;
mod exec;
mod export;
mod favorite;
mod get;
mod history;
mod import;
mod lint;
mod list;
mod output;
mod remove;
mod render;
mod search;
mod update;

use crate::{
    cli::{Command, CompletionsArgs, UpdateArgs},
    db::{Database, Prompt},
    error::Result,
    prompt::PromptDocument,
};

pub fn completions(arguments: CompletionsArgs) -> Result<()> {
    completions::run(arguments)
}

pub fn update(arguments: UpdateArgs) -> Result<()> {
    update::run(arguments)
}

pub fn execute(command: Command, database: &mut Database) -> Result<()> {
    match command {
        Command::Add(arguments) => add::run(arguments, database),
        Command::Edit(arguments) => edit::run(arguments, database),
        Command::Rm(arguments) => remove::run(arguments, database),
        Command::Get(arguments) => get::run(arguments, database),
        Command::Exec(arguments) => exec::run(arguments, database),
        Command::List(arguments) => list::run(arguments, database),
        Command::Lint(arguments) => lint::run(arguments, database),
        Command::Search(arguments) => search::run(arguments, database),
        Command::Import(arguments) => import::run(arguments, database),
        Command::Export(arguments) => export::run(arguments, database),
        Command::Favorite(arguments) => favorite::run(arguments, database),
        Command::History(arguments) => history::run(arguments, database),
        Command::Completions(_) | Command::Update(_) => {
            unreachable!("standalone command reached database dispatcher")
        }
    }
}

fn warn_single_brace_variables(content: &str) {
    for expression in crate::prompt::template::single_brace_variables(content) {
        eprintln!(
            "warning: `{expression}` uses single braces and will not be replaced; use `{{{expression}}}` for a template variable"
        );
    }
}

fn prompt_to_document(prompt: Prompt) -> PromptDocument {
    PromptDocument {
        name: prompt.name,
        description: prompt.description,
        tags: prompt.tags,
        exec: prompt.exec,
        content: prompt.content,
    }
}
