pub mod markdown;
pub mod template;

use std::collections::BTreeSet;

use crate::error::{Error, Result};

pub fn parse_exec_command(command: &str) -> Result<Vec<String>> {
    let arguments = shell_words::split(command)
        .map_err(|error| Error::Message(format!("invalid exec command: {error}")))?;
    if arguments.is_empty() {
        return Err(Error::Message("exec command must not be empty".into()));
    }
    Ok(arguments)
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Message("prompt name must not be empty".into()));
    }
    if !name.split('/').all(is_valid_name_segment) {
        return Err(Error::Message(format!("invalid prompt name: {name}")));
    }
    Ok(())
}

pub fn validate_group(group: &str) -> Result<()> {
    let Some(name) = group.strip_suffix('/') else {
        return Err(Error::Message(format!(
            "prompt group must end with '/': {group}"
        )));
    };
    validate_name(name).map_err(|_| Error::Message(format!("invalid prompt group: {group}")))
}

fn is_valid_name_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    first.is_ascii_alphanumeric()
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

pub fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            return Err(Error::Message("tag must not be empty".into()));
        }
        normalized.insert(tag.to_owned());
    }
    Ok(normalized.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_prompt_names() {
        for name in [
            "code-review",
            "typescript.review",
            "debug_log",
            "A1",
            "work/week-report",
            "work/reports/weekly",
        ] {
            validate_name(name).unwrap();
        }
        for name in [
            "",
            "has space",
            "/path",
            "path/",
            "work//report",
            "_starts-wrong",
            "shell;code",
        ] {
            assert!(validate_name(name).is_err(), "{name} should be invalid");
        }
    }

    #[test]
    fn validates_prompt_groups() {
        for group in ["work/", "work/reports/"] {
            validate_group(group).unwrap();
        }
        for group in ["", "work", "/", "work//"] {
            assert!(validate_group(group).is_err(), "{group} should be invalid");
        }
    }

    #[test]
    fn normalizes_and_sorts_tags() {
        assert_eq!(
            normalize_tags(vec![" review ".into(), "coding".into(), "review".into()]).unwrap(),
            vec!["coding", "review"]
        );
    }

    #[test]
    fn parses_exec_commands_without_a_shell() {
        assert_eq!(
            parse_exec_command("codex exec --model 'gpt 5' -").unwrap(),
            ["codex", "exec", "--model", "gpt 5", "-"]
        );
        assert!(parse_exec_command("  ").is_err());
        assert!(parse_exec_command("codex '").is_err());
    }
}
