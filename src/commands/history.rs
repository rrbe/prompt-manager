use anstyle::{AnsiColor, Style};
use similar::TextDiff;

use crate::{
    cli::{HistoryArgs, HistoryCommand, HistoryDiffArgs},
    db::{Database, PromptVersion},
    error::Result,
    prompt::{
        markdown::{self, PromptDocument},
        validate_name,
    },
};

use super::{
    current_local_offset, format_local_timestamp, format_table, stdout_supports_color, style_text,
    write_stdout,
};

pub fn run(arguments: HistoryArgs, database: &Database) -> Result<()> {
    validate_name(&arguments.name)?;
    match arguments.command {
        Some(HistoryCommand::Diff(diff)) => diff_versions(&arguments.name, diff, database),
        None => list_versions(&arguments.name, database),
    }
}

fn list_versions(name: &str, database: &Database) -> Result<()> {
    const HEADERS: [&str; 3] = ["VERSION", "CREATED AT", "NAME"];

    let versions = database.prompt_history(name)?;
    if versions.is_empty() {
        return Ok(());
    }
    let local_offset = current_local_offset()?;
    let rows = versions
        .into_iter()
        .map(|version| {
            Ok([
                version.version.to_string(),
                format_local_timestamp(version.created_at, local_offset)?,
                version.name,
            ])
        })
        .collect::<Result<Vec<_>>>()?;
    write_stdout(&format_table(
        &HEADERS,
        &rows,
        &[
            Style::new().dimmed(),
            Style::new().dimmed(),
            AnsiColor::Cyan.on_default(),
        ],
        stdout_supports_color(),
    ))
}

fn diff_versions(name: &str, arguments: HistoryDiffArgs, database: &Database) -> Result<()> {
    let old = database.prompt_version(name, arguments.old)?;
    let new = database.prompt_version(name, arguments.new)?;
    let old_markdown = markdown::export(&document(old))?;
    let new_markdown = markdown::export(&document(new))?;
    let output = TextDiff::from_lines(&old_markdown, &new_markdown)
        .unified_diff()
        .header(
            &format!("{name}@{}", arguments.old),
            &format!("{name}@{}", arguments.new),
        )
        .to_string();
    write_stdout(&colorize_diff(&output, stdout_supports_color()))
}

fn colorize_diff(diff: &str, colors_enabled: bool) -> String {
    if !colors_enabled {
        return diff.to_owned();
    }

    diff.split_inclusive('\n')
        .map(|line| {
            let (content, newline) = line
                .strip_suffix('\n')
                .map_or((line, ""), |content| (content, "\n"));
            let style = if content.starts_with("--- ") {
                AnsiColor::Red.on_default().bold()
            } else if content.starts_with("+++ ") {
                AnsiColor::Green.on_default().bold()
            } else if content.starts_with("@@") {
                AnsiColor::Cyan.on_default()
            } else if content.starts_with('-') {
                AnsiColor::Red.on_default()
            } else if content.starts_with('+') {
                AnsiColor::Green.on_default()
            } else {
                Style::new()
            };
            format!("{}{newline}", style_text(content, style, true))
        })
        .collect()
}

fn document(version: PromptVersion) -> PromptDocument {
    PromptDocument {
        name: version.name,
        description: version.description,
        tags: version.tags,
        exec: version.exec,
        content: version.content,
    }
}

#[cfg(test)]
mod tests {
    use super::colorize_diff;

    #[test]
    fn colors_diff_lines_by_meaning() {
        let diff = "--- old\n+++ new\n@@ -1 +1 @@\n-old\n+new\n unchanged\n";
        let colored = colorize_diff(diff, true);

        assert!(colored.contains("\u{1b}[1m\u{1b}[31m--- old\u{1b}[0m"));
        assert!(colored.contains("\u{1b}[1m\u{1b}[32m+++ new\u{1b}[0m"));
        assert!(colored.contains("\u{1b}[36m@@ -1 +1 @@\u{1b}[0m"));
        assert!(colored.contains("\u{1b}[31m-old\u{1b}[0m"));
        assert!(colored.contains("\u{1b}[32m+new\u{1b}[0m"));
        assert!(colored.ends_with(" unchanged\n"));
    }

    #[test]
    fn leaves_diff_plain_when_colors_are_disabled() {
        let diff = "--- old\n+++ new\n-old\n+new\n";

        assert_eq!(colorize_diff(diff, false), diff);
    }
}
