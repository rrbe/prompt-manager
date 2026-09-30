use crate::{
    cli::AddArgs,
    db::Database,
    editor,
    error::{Error, Result},
    prompt::{PromptDocument, markdown, validate_name},
    stdin,
};

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
    let document = if arguments.no_edit {
        document.normalize()?
    } else {
        let initial = markdown::export_for_editor(&document)?;
        editor::edit_until_valid(&initial, markdown::parse_editor)?
    };
    if document.content.trim().is_empty() {
        anstream::eprintln!();
        super::diagnostics::print_group(
            &document.name,
            &[super::diagnostics::Diagnostic {
                warning: true,
                message: "Prompt was not created: content is empty.".into(),
                suggestion: None,
            }],
        );
        return Ok(());
    }
    super::warn_single_brace_variables(&document.name, &document.content);
    database.create_prompt(&document)
}
