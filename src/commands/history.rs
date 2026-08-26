use similar::TextDiff;

use crate::{
    cli::{HistoryArgs, HistoryCommand, HistoryDiffArgs},
    db::{Database, PromptVersion},
    error::Result,
    prompt::{
        markdown::{self, PromptDocument},
        validate_name,
    },
};

use super::{current_local_offset, format_local_timestamp, format_table, write_stdout};

pub fn run(arguments: HistoryArgs, database: &Database) -> Result<()> {
    validate_name(&arguments.name)?;
    match arguments.command {
        Some(HistoryCommand::Diff(diff)) => diff_versions(&arguments.name, diff, database),
        None => list_versions(&arguments.name, database),
    }
}

fn list_versions(name: &str, database: &Database) -> Result<()> {
    const HEADERS: [&str; 3] = ["VERSION", "CREATED AT", "NAME"];

    let versions = database.prompt_history(name)?;
    if versions.is_empty() {
        return Ok(());
    }
    let local_offset = current_local_offset()?;
    let rows = versions
        .into_iter()
        .map(|version| {
            Ok([
                version.version.to_string(),
                format_local_timestamp(version.created_at, local_offset)?,
                version.name,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    write_stdout(&format_table(&HEADERS, &rows))
}

fn diff_versions(name: &str, arguments: HistoryDiffArgs, database: &Database) -> Result<()> {
    let old = database.prompt_version(name, arguments.old)?;
    let new = database.prompt_version(name, arguments.new)?;
    let old_markdown = markdown::export(&document(old))?;
    let new_markdown = markdown::export(&document(new))?;
    let output = TextDiff::from_lines(&old_markdown, &new_markdown)
        .unified_diff()
        .header(
            &format!("{name}@{}", arguments.old),
            &format!("{name}@{}", arguments.new),
        )
        .to_string();
    write_stdout(&output)
}

fn document(version: PromptVersion) -> PromptDocument {
    PromptDocument {
        name: version.name,
        description: version.description,
        tags: version.tags,
        content: version.content,
    }
}
