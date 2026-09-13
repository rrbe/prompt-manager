use std::io::{self, Write};

use crate::{cli::RemoveArgs, db::Database, error::Result, prompt::validate_name};

pub fn run(arguments: RemoveArgs, database: &mut Database) -> Result<()> {
    for name in &arguments.names {
        validate_name(name)?;
        // Resolve the target before asking for destructive confirmation.
        database.get_prompt(name)?;

        if arguments.force || confirm(name)? {
            database.delete_prompt(name)?;
        }
    }
    Ok(())
}

fn confirm(name: &str) -> Result<bool> {
    let stdin = io::stdin();
    let stderr = io::stderr();
    let mut error_output = stderr.lock();
    write!(error_output, "Remove prompt '{name}'? [y/N] ")?;
    error_output.flush()?;

    let mut response = String::new();
    stdin.read_line(&mut response)?;
    Ok(matches!(
        response.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
