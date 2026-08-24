use std::fs;

use crate::{
    cli::ImportArgs,
    db::Database,
    error::{Error, Result},
    prompt::markdown,
};

use super::document_to_input;

pub fn run(arguments: ImportArgs, database: &mut Database) -> Result<()> {
    let bytes = fs::read(&arguments.file).map_err(|source| Error::ReadFile {
        path: arguments.file.clone(),
        source,
    })?;
    let source = String::from_utf8(bytes).map_err(|_| {
        Error::Message(format!(
            "Markdown prompt is not valid UTF-8: {}",
            arguments.file.display()
        ))
    })?;
    let document = markdown::parse(&source)?;
    database.create_prompt(&document_to_input(document))
}
