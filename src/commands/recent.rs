use crate::{db::Database, error::Result};

use super::write_stdout;

pub fn run(database: &Database) -> Result<()> {
    let names = database.recent_prompt_names()?;
    if names.is_empty() {
        return Ok(());
    }
    write_stdout(&format!("{}\n", names.join("\n")))
}
