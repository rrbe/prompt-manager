use crate::{
    cli::NameArgs,
    db::Database,
    editor,
    error::Result,
    prompt::{markdown, validate_name},
};

use super::{document_to_input, prompt_to_document};

pub fn run(arguments: NameArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    let prompt = database.get_prompt(&arguments.name)?;
    let initial = markdown::export(&prompt_to_document(prompt))?;
    let document = editor::edit_until_valid(&initial, markdown::parse)?;
    database.update_prompt(&arguments.name, &document_to_input(document))
}
