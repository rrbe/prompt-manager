use similar::TextDiff;

use crate::{
    cli::{DiffArgs, VersionReference},
    db::{Database, PromptVersion},
    error::Result,
    prompt::markdown::{self, PromptDocument},
};

use super::write_stdout;

pub fn run(arguments: DiffArgs, database: &Database) -> Result<()> {
    let old = database.prompt_version(&arguments.old.name, arguments.old.version)?;
    let new = database.prompt_version(&arguments.new.name, arguments.new.version)?;
    let old_markdown = markdown::export(&document(old))?;
    let new_markdown = markdown::export(&document(new))?;
    let output = TextDiff::from_lines(&old_markdown, &new_markdown)
        .unified_diff()
        .header(&reference(&arguments.old), &reference(&arguments.new))
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

fn reference(reference: &VersionReference) -> String {
    format!("{}@{}", reference.name, reference.version)
}
