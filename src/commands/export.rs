use std::{fs, io::Write};

use tempfile::NamedTempFile;

use crate::{
    cli::ExportArgs,
    db::Database,
    error::{Error, Result},
    prompt::{markdown, validate_name},
};

use super::{prompt_to_document, write_stdout};

pub fn run(arguments: ExportArgs, database: &mut Database) -> Result<()> {
    if arguments.all {
        return export_all(arguments, database);
    }

    let name = arguments
        .target
        .to_str()
        .ok_or_else(|| Error::Message("prompt name is not valid UTF-8".into()))?;
    validate_name(name)?;
    let prompt = database.get_prompt(name)?;
    let markdown = markdown::export(&prompt_to_document(prompt))?;
    write_stdout(&markdown)
}

fn export_all(arguments: ExportArgs, database: &mut Database) -> Result<()> {
    fs::create_dir_all(&arguments.target)?;
    let names = database.list_prompt_names()?;
    for name in names {
        let prompt = database.get_prompt(&name)?;
        let markdown = markdown::export(&prompt_to_document(prompt))?;
        let destination = arguments.target.join(format!("{name}.md"));
        atomic_write(&arguments.target, &destination, &markdown)?;
    }
    Ok(())
}

fn atomic_write(
    directory: &std::path::Path,
    destination: &std::path::Path,
    value: &str,
) -> Result<()> {
    let mut temporary = NamedTempFile::new_in(directory)?;
    temporary.write_all(value.as_bytes())?;
    temporary.flush()?;
    temporary
        .persist(destination)
        .map_err(|error| Error::Io(error.error))?;
    Ok(())
}
