use crate::{cli::NameArgs, db::Database, error::Result, prompt::validate_name};

use super::{list::format_timestamp, write_stdout};

pub fn run(arguments: NameArgs, database: &Database) -> Result<()> {
    validate_name(&arguments.name)?;
    let versions = database.prompt_history(&arguments.name)?;
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
