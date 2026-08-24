use std::{collections::HashMap, fs};

use crate::{
    cli::RenderArgs,
    db::Database,
    error::{Error, Result},
    prompt::{template, validate_name},
    stdin,
};

use super::write_stdout;

pub fn run(arguments: RenderArgs, database: &mut Database) -> Result<()> {
    validate_name(&arguments.name)?;
    let prompt = database.get_prompt(&arguments.name)?;
    let content = expand_compositions(database, &prompt.content, &mut vec![prompt.name.clone()])?;
    let mut values = HashMap::new();

    for variable in arguments.variables {
        insert_value(&mut values, variable.key, variable.value)?;
    }
    for file in arguments.files {
        if values.contains_key(&file.key) {
            return Err(Error::DuplicateVariableSource(file.key));
        }
        let bytes = fs::read(&file.path).map_err(|source| Error::ReadFile {
            path: file.path.clone(),
            source,
        })?;
        let value = String::from_utf8(bytes).map_err(|_| {
            Error::Message(format!("file is not valid UTF-8: {}", file.path.display()))
        })?;
        values.insert(file.key, value);
    }

    let needs_input = template::placeholders(&content)
        .iter()
        .any(|placeholder| placeholder.name == "input");
    if needs_input && !values.contains_key("input") {
        values.insert("input".into(), stdin::read_piped_input()?);
    }

    let rendered = template::render(&content, &values)?;
    database.mark_prompt_used(&prompt.name)?;
    write_stdout(&rendered)
}

fn expand_compositions(
    database: &mut Database,
    content: &str,
    stack: &mut Vec<String>,
) -> Result<String> {
    let compositions = template::compositions(content);
    let mut output = String::with_capacity(content.len());
    let mut cursor = 0;

    for composition in compositions {
        output.push_str(&content[cursor..composition.start]);
        if let Some(cycle_start) = stack.iter().position(|name| name == &composition.name) {
            let mut cycle = stack[cycle_start..].to_vec();
            cycle.push(composition.name);
            return Err(Error::Message(format!(
                "prompt composition cycle: {}",
                cycle.join(" -> ")
            )));
        }

        let prompt = database.get_prompt(&composition.name)?;
        stack.push(prompt.name);
        let expanded = expand_compositions(database, &prompt.content, stack)?;
        stack.pop();
        output.push_str(&expanded);
        cursor = composition.end;
    }
    output.push_str(&content[cursor..]);
    Ok(output)
}

fn insert_value(values: &mut HashMap<String, String>, key: String, value: String) -> Result<()> {
    if values.insert(key.clone(), value).is_some() {
        return Err(Error::DuplicateVariableSource(key));
    }
    Ok(())
}
