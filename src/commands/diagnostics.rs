use anstyle::{AnsiColor, Style};

use crate::prompt::template;

pub(super) fn location(content: &str, start: usize) -> String {
    let prefix = &content[..start];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    format!("line {line}, column {column}")
}

pub(super) struct Diagnostic {
    pub(super) warning: bool,
    pub(super) message: String,
    pub(super) suggestion: Option<String>,
}

pub(super) fn single_brace_diagnostics(content: &str) -> Vec<Diagnostic> {
    let mut cursor = 0;
    template::single_brace_variables(content)
        .into_iter()
        .map(|expression| {
            let start = cursor + content[cursor..].find(expression).unwrap();
            cursor = start + expression.len();
            Diagnostic {
                warning: true,
                message: format!("Single-brace variable at {}", location(content, start)),
                suggestion: Some(format!("{expression}  →  {{{expression}}}")),
            }
        })
        .collect()
}

pub(super) fn print_group(name: &str, issues: &[Diagnostic]) {
    const NAME: Style = AnsiColor::Cyan.on_default().bold();
    const MUTED: Style = Style::new().dimmed();
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
