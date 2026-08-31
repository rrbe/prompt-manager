use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
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

    if arguments.interactive {
        let input = io::stdin();
        let output = io::stderr();
        prompt_for_variables(&content, &mut values, input.lock(), output.lock())?;
    } else {
        let needs_input = template::placeholders(&content)
            .iter()
            .any(|placeholder| placeholder.name == "input");
        if needs_input && !values.contains_key("input") {
            values.insert("input".into(), stdin::read_piped_input()?);
        }
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

fn prompt_for_variables(
    content: &str,
    values: &mut HashMap<String, String>,
    mut input: impl BufRead,
    mut output: impl Write,
) -> Result<()> {
    let mut names = Vec::new();
    for placeholder in template::placeholders(content) {
        if !values.contains_key(&placeholder.name) && !names.contains(&placeholder.name) {
            names.push(placeholder.name);
        }
    }

    let total = names.len();
    for (index, name) in names.into_iter().enumerate() {
        writeln!(output, "[{}/{}] {name}", index + 1, total)?;
        writeln!(
            output,
            "Enter or paste the value. Finish with a line containing only `EOF`."
        )?;
        output.flush()?;

        let mut value = String::new();
        loop {
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                return Err(Error::Message(format!(
                    "interactive input ended before completing variable: {name}"
                )));
            }
            if trim_line_ending(&line) == "EOF" {
                break;
            }
            value.push_str(&line);
        }
        values.insert(name, value.trim().to_owned());
    }

    Ok(())
}

fn trim_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_for_distinct_missing_variables_in_template_order() {
        let mut values = HashMap::from([("provided".into(), "existing".into())]);
        let mut output = Vec::new();

        prompt_for_variables(
            "{{provided}} {{first}} {{second}} {{first}}",
            &mut values,
            &b"one\nEOF\ntwo\nlines\nEOF\n"[..],
            &mut output,
        )
        .unwrap();

        assert_eq!(values["provided"], "existing");
        assert_eq!(values["first"], "one");
        assert_eq!(values["second"], "two\nlines");
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "[1/2] first\nEnter or paste the value. Finish with a line containing only `EOF`.\n[2/2] second\nEnter or paste the value. Finish with a line containing only `EOF`.\n"
        );
    }

    #[test]
    fn trims_surrounding_whitespace_from_interactive_values() {
        let mut values = HashMap::new();

        prompt_for_variables(
            "{{value}}",
            &mut values,
            &b"\n  first line\nsecond line  \n\nEOF\n"[..],
            Vec::new(),
        )
        .unwrap();

        assert_eq!(values["value"], "first line\nsecond line");
    }

    #[test]
    fn reports_incomplete_interactive_input() {
        let error = prompt_for_variables(
            "{{value}}",
            &mut HashMap::new(),
            &b"unfinished\n"[..],
            Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "interactive input ended before completing variable: value"
        );
    }
}
