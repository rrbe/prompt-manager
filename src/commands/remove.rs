use std::io::{self, IsTerminal, Write};

use crate::{
    cli::RemoveArgs,
    db::Database,
    error::{Error, Result},
    prompt::validate_name,
};

pub fn run(arguments: RemoveArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    // Resolve the target before asking for destructive confirmation.
    database.get_prompt(&arguments.name)?;

    if !arguments.force && !confirm(&arguments.name)? {
        return Ok(());
    }
    database.delete_prompt(&arguments.name)
}

fn confirm(name: &str) -> Result<bool> {
    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return Err(Error::Message(
            "refusing to prompt for confirmation without a TTY; use --force".into(),
        ));
    }

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
