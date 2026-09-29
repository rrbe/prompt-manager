use super::output::write_stdout;
use crate::{cli::GetArgs, db::Database, error::Result};

pub fn run(arguments: GetArgs, database: &mut Database) -> Result<()> {
    let rendered = super::render::render(arguments.prompt, database)?;
    database.mark_prompt_used(&rendered.name)?;
    write_stdout(&rendered.content)
}
