use crate::{
    cli::AddArgs,
    db::Database,
    editor,
    error::{Error, Result},
    prompt::{
        markdown::{self, PromptDocument},
        validate_name,
    },
    stdin,
};

use super::document_to_input;

pub fn run(arguments: AddArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    if database.prompt_exists(&arguments.name)? {
        return Err(Error::PromptAlreadyExists(arguments.name));
    }
    let mut document = PromptDocument {
        name: arguments.name,
        description: None,
        tags: Vec::new(),
        exec: None,
        content: String::new(),
    };
    if let Some(content) = stdin::read_piped_input_if_available()? {
        document.content = content;
    }
    let initial = markdown::export(&document)?;
    let document = if arguments.no_edit {
        markdown::parse(&initial)?
    } else {
        editor::edit_until_valid(&initial, markdown::parse)?
    };
    database.create_prompt(&document_to_input(document))
}
