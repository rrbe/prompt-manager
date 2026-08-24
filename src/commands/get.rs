use std::{
    collections::HashMap,
    fs,
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    cli::GetArgs,
    db::Database,
    error::{Error, Result},
    prompt::{template, validate_name},
    stdin,
};

use super::write_stdout;

pub fn run(arguments: GetArgs, database: &mut Database) -> Result<()> {
    let name = if arguments.pick {
        pick(database)?
    } else if let Some(id) = arguments.id {
        database.prompt_name_by_id(id)?
    } else {
        arguments
            .name
            .expect("clap requires a prompt name unless --id or --pick is used")
    };
    validate_name(&name)?;
    let prompt = database.get_prompt(&name)?;
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

fn pick(database: &Database) -> Result<String> {
    let names = database.list_prompt_names()?;
    if names.is_empty() {
        return Err(Error::Message("no prompts available to pick".into()));
    }

    let mut child = Command::new("fzf")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::Message("fzf is not installed or not in PATH".into())
            } else {
                Error::Io(error)
            }
        })?;

    if let Some(mut input) = child.stdin.take() {
        let candidates = format!("{}\n", names.join("\n"));
        if let Err(error) = input.write_all(candidates.as_bytes())
            && error.kind() != std::io::ErrorKind::BrokenPipe
        {
            return Err(error.into());
        }
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Message("fzf exited without a selection".into()));
    }
    let selection = String::from_utf8(output.stdout)
        .map_err(|_| Error::Message("fzf returned a non-UTF-8 selection".into()))?;
    let selection = selection.trim_end_matches(['\r', '\n']);
    if selection.is_empty() || !names.iter().any(|name| name == selection) {
        return Err(Error::Message("fzf returned an invalid selection".into()));
    }

    Ok(selection.to_owned())
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
