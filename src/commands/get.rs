use crate::{cli::NameArgs, db::Database, error::Result, prompt::validate_name};

use super::write_stdout;

pub fn run(arguments: NameArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    let prompt = database.get_prompt_and_mark_used(&arguments.name)?;
    write_stdout(&prompt.content)
}
