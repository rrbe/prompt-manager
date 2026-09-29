use std::collections::{BTreeMap, HashSet};

use anstyle::{AnsiColor, Style};

use crate::{
    cli::LintArgs,
    db::Database,
    error::{Error, Result},
    prompt::{template, validate_name},
};

pub fn run(arguments: LintArgs, database: &mut Database) -> Result<()> {
    let names = if arguments.all {
        database.list_prompt_names()?
    } else {
        let name = arguments.name.expect("clap requires a name or --all");
        validate_name(&name)?;
        vec![name]
    };
    let mut errors = 0;
    let mut warnings = 0;
    let mut checked = HashSet::new();
    let mut diagnostics = BTreeMap::new();
    for name in &names {
        inspect(
            database,
            name,
            &mut checked,
            &mut errors,
            &mut warnings,
            &mut diagnostics,
        )?;
    }
    for name in &names {
        let prompt = database.get_prompt(name)?;
        match super::get::expand_compositions(database, &prompt.content, &mut vec![name.clone()]) {
            Ok(content) => {
                // Source syntax was checked separately; expanded positions refer to the combined body.
                if content != prompt.content
                    && let Err(error) = template::validate(&content)
                {
                    report(
                        &mut diagnostics,
                        name,
                        false,
                        format!("Expanded content: {error}"),
                        None,
                    );
                    errors += 1;
                }
            }
            Err(Error::Message(message)) => {
                report(&mut diagnostics, name, false, message, None);
                errors += 1;
            }
            // Missing references were reported at their source by inspect.
            Err(Error::PromptNotFound(_)) => {}
            Err(error) => return Err(error),
        }
    }
    print_report(&diagnostics, checked.len(), errors, warnings);
    if errors > 0 {
        return Err(Error::LintFailed);
    }
    Ok(())
}

fn inspect(
    database: &mut Database,
    name: &str,
    checked: &mut HashSet<String>,
    errors: &mut usize,
    warnings: &mut usize,
    diagnostics: &mut BTreeMap<String, Vec<Diagnostic>>,
) -> Result<()> {
    if !checked.insert(name.to_owned()) {
        return Ok(());
    }
    let prompt = database.get_prompt(name)?;
    if let Err(error) = template::validate(&prompt.content) {
        report(diagnostics, name, false, error.to_string(), None);
        *errors += 1;
    }
    if prompt.content.trim().is_empty() {
        report(diagnostics, name, true, "Content is empty".into(), None);
        *warnings += 1;
    }
    let mut cursor = 0;
    for expression in template::single_brace_variables(&prompt.content) {
        let start = cursor + prompt.content[cursor..].find(expression).unwrap();
        let location = location(&prompt.content, start);
        report(
            diagnostics,
            name,
            true,
            format!("Single-brace variable at {location}"),
            Some(format!("{expression}  →  {{{expression}}}")),
        );
        *warnings += 1;
        cursor = start + expression.len();
    }
    for reference in template::compositions(&prompt.content) {
        if !database.prompt_exists(&reference.name)? {
            let location = location(&prompt.content, reference.start);
            report(
                diagnostics,
                name,
                false,
                format!("Prompt not found: {} ({location})", reference.name),
                None,
            );
            *errors += 1;
        } else {
            inspect(
                database,
                &reference.name,
                checked,
                errors,
                warnings,
                diagnostics,
            )?;
        }
    }
    Ok(())
}

fn location(content: &str, start: usize) -> String {
    let prefix = &content[..start];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    format!("line {line}, column {column}")
}

struct Diagnostic {
    warning: bool,
    message: String,
    suggestion: Option<String>,
}

fn report(
    diagnostics: &mut BTreeMap<String, Vec<Diagnostic>>,
    name: &str,
    warning: bool,
    message: String,
    suggestion: Option<String>,
) {
    diagnostics
        .entry(name.to_owned())
        .or_default()
        .push(Diagnostic {
            warning,
            message,
            suggestion,
        });
}

fn print_report(
    diagnostics: &BTreeMap<String, Vec<Diagnostic>>,
    checked: usize,
    errors: usize,
    warnings: usize,
) {
    const NAME: Style = AnsiColor::Cyan.on_default().bold();
    const MUTED: Style = Style::new().dimmed();
    anstream::eprintln!();
    for (name, issues) in diagnostics {
        anstream::eprintln!("{NAME}{name}{NAME:#}");
        for issue in issues {
            let (label, color) = if issue.warning {
                ("Warning", AnsiColor::Yellow)
            } else {
                ("Error", AnsiColor::Red)
            };
            let style = color.on_default().bold();
            anstream::eprintln!("  {style}{label}{style:#}  {}", issue.message);
            if let Some(suggestion) = &issue.suggestion {
                anstream::eprintln!("           {MUTED}Replace:{MUTED:#} {suggestion}");
            }
        }
        anstream::eprintln!();
    }
    let status = if errors > 0 {
        AnsiColor::Red
    } else if warnings > 0 {
        AnsiColor::Yellow
    } else {
        AnsiColor::Green
    }
    .on_default()
    .bold();
    let prompts = if checked == 1 { "prompt" } else { "prompts" };
    let error_label = if errors == 1 { "error" } else { "errors" };
    let warning_label = if warnings == 1 { "warning" } else { "warnings" };
    anstream::eprintln!(
        "{MUTED}Checked {checked} {prompts}{MUTED:#} · {status}{errors} {error_label} · {warnings} {warning_label}{status:#}"
    );
    anstream::eprintln!();
}
