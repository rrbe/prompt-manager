use crate::{
    cli::AddArgs,
    db::Database,
    editor,
    error::{Error, Result},
    prompt::{
        markdown::{self, PromptDocument},
        validate_name,
    },
};

use super::document_to_input;

pub fn run(arguments: AddArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    if database.prompt_exists(&arguments.name)? {
        return Err(Error::PromptAlreadyExists(arguments.name));
    }
    let initial = markdown::export(&PromptDocument {
        name: arguments.name,
        description: None,
        tags: Vec::new(),
        content: String::new(),
    })?;
    let document = editor::edit_until_valid(&initial, markdown::parse)?;
    database.create_prompt(&document_to_input(document))
}
