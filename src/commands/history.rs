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

use super::{list::format_timestamp, write_stdout};

pub fn run(arguments: HistoryArgs, database: &Database) -> Result<()> {
    validate_name(&arguments.name)?;
    match arguments.command {
        Some(HistoryCommand::Diff(diff)) => diff_versions(&arguments.name, diff, database),
        None => list_versions(&arguments.name, database),
    }
}

fn list_versions(name: &str, database: &Database) -> Result<()> {
    let versions = database.prompt_history(name)?;
    let lines = versions
        .into_iter()
        .map(|version| {
            Ok(format!(
                "{}\t{}\t{}",
                version.version,
                format_timestamp(version.created_at)?,
                version.name
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    if lines.is_empty() {
        return Ok(());
    }
    write_stdout(&format!("{}\n", lines.join("\n")))
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
