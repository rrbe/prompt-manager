use crate::{cli::FavoriteArgs, db::Database, error::Result, prompt::validate_name};

pub fn run(arguments: FavoriteArgs, database: &Database) -> Result<()> {
    validate_name(&arguments.name)?;
    database.set_prompt_favorite(&arguments.name, !arguments.remove)
}
