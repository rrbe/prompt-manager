use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    error::{Error, Result},
    prompt::{normalize_tags, parse_exec_command, template, validate_name},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptDocument {
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub exec: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_tags")]
    tags: Vec<String>,
    #[serde(default)]
    exec: Option<String>,
}

const FIELD_HELP_COLUMN: usize = 20;
const CONTENT_PLACEHOLDER: &str = "<!-- Enter prompt content here. -->";

fn deserialize_optional_tags<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

pub fn parse(source: &str) -> Result<PromptDocument> {
    let (metadata_source, body) = split_front_matter(source)?;
    let metadata: Metadata = serde_yaml::from_str(metadata_source)?;
    validate_name(&metadata.name)?;
    let tags = normalize_tags(metadata.tags)?;
    if let Some(command) = &metadata.exec {
        parse_exec_command(command)?;
    }
    let body_start = source.len() - body.len();
    let body_line_offset = source[..body_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    template::validate_with_line_offset(body, body_line_offset)?;

    Ok(PromptDocument {
        name: metadata.name,
        description: metadata.description,
        tags,
        exec: metadata.exec,
        content: body.to_owned(),
    })
}

pub fn export(document: &PromptDocument) -> Result<String> {
    validate_name(&document.name)?;
    if let Some(command) = &document.exec {
        parse_exec_command(command)?;
    }
    let metadata = Metadata {
        name: document.name.clone(),
        description: document.description.clone(),
        tags: normalize_tags(document.tags.clone())?,
        exec: document.exec.clone(),
    };
    let yaml = serde_yaml::to_string(&metadata)?
        .replace("description: null\n", "description:\n")
        .replace("exec: null\n", "exec:\n");
    let yaml = add_field_help(&yaml);
    Ok(format!("---\n{yaml}---\n\n{}", document.content))
}

fn add_field_help(yaml: &str) -> String {
    let mut output = String::new();
    for line in yaml.lines() {
        let (line, help, show_tag_examples) = match line {
            line if line.starts_with("name:") => (line, Some("required, unique"), false),
            line if line.starts_with("description:") => {
                (line, Some("optional, short description"), false)
            }
            "tags: []" => ("tags:", Some("optional, YAML list"), true),
            "tags:" => (line, Some("optional, YAML list"), false),
            line if line.starts_with("exec:") => (line, Some("optional, e.g. codex exec -"), false),
            _ => (line, None, false),
        };
        output.push_str(line);
        if let Some(help) = help {
            let padding = FIELD_HELP_COLUMN
                .saturating_sub(line.chars().count())
                .max(2);
            output.push_str(&" ".repeat(padding));
            output.push_str("# ");
            output.push_str(help);
        }
        output.push('\n');
        if show_tag_examples {
            output.push_str("  # - coding\n  # - review\n");
        }
    }
    output
}

pub fn export_for_editor(document: &PromptDocument) -> Result<String> {
    let markdown = export(document)?;
    if document.content.is_empty() {
        return Ok(format!("{markdown}{CONTENT_PLACEHOLDER}"));
    }
    Ok(markdown)
}

pub fn parse_editor(source: &str) -> Result<PromptDocument> {
    let mut document = parse(source)?;
    document.content = document
        .content
        .split_inclusive('\n')
        .filter(|line| trim_line_ending(line) != CONTENT_PLACEHOLDER)
        .collect();
    Ok(document)
}

fn split_front_matter(source: &str) -> Result<(&str, &str)> {
    let first_line_end = line_end(source, 0).ok_or_else(|| {
        Error::Message("invalid Markdown prompt: missing opening front matter delimiter".into())
    })?;
    if trim_line_ending(&source[..first_line_end]) != "---" {
        return Err(Error::Message(
            "invalid Markdown prompt: missing opening front matter delimiter".into(),
        ));
    }

    let metadata_start = first_line_end;
    let mut cursor = metadata_start;
    loop {
        let end = line_end(source, cursor).unwrap_or(source.len());
        let line = trim_line_ending(&source[cursor..end]);
        if line == "---" {
            let mut body_start = end;
            if source[body_start..].starts_with("\r\n") {
                body_start += 2;
            } else if source[body_start..].starts_with('\n') {
                body_start += 1;
            }
            return Ok((&source[metadata_start..cursor], &source[body_start..]));
        }
        if end == source.len() {
            return Err(Error::Message(
                "invalid Markdown prompt: missing closing front matter delimiter".into(),
            ));
        }
        cursor = end;
    }
}

fn line_end(source: &str, start: usize) -> Option<usize> {
    source[start..]
        .find('\n')
        .map(|relative| start + relative + 1)
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
    fn parses_front_matter_and_preserves_body() {
        let document = parse(
            "---\nname: code-review\ndescription: Review source code\ntags:\n  - review\n  - coding\nexec: codex exec -\n---\n\nReview:\n\n{{input}}\n",
        )
        .unwrap();
        assert_eq!(document.name, "code-review");
        assert_eq!(document.tags, vec!["coding", "review"]);
        assert_eq!(document.exec.as_deref(), Some("codex exec -"));
        assert_eq!(document.content, "Review:\n\n{{input}}\n");
    }

    #[test]
    fn export_round_trips_without_adding_a_body_newline() {
        let document = PromptDocument {
            name: "test".into(),
            description: None,
            tags: vec!["z".into(), "a".into()],
            exec: Some("codex exec -".into()),
            content: "body without newline".into(),
        };
        let markdown = export(&document).unwrap();
        assert_eq!(
            parse(&markdown).unwrap(),
            PromptDocument {
                tags: vec!["a".into(), "z".into()],
                ..document
            }
        );
    }

    #[test]
    fn export_includes_field_help_and_blank_optional_fields() {
        let document = PromptDocument {
            name: "test".into(),
            description: None,
            tags: Vec::new(),
            exec: None,
            content: "body".into(),
        };

        assert_eq!(
            export(&document).unwrap(),
            "---\nname: test          # required, unique\ndescription:        # optional, short description\ntags:               # optional, YAML list\n  # - coding\n  # - review\nexec:               # optional, e.g. codex exec -\n---\n\nbody"
        );
    }

    #[test]
    fn editor_document_uses_a_non_persistent_content_placeholder() {
        let document = PromptDocument {
            name: "test".into(),
            description: None,
            tags: Vec::new(),
            exec: None,
            content: String::new(),
        };

        let markdown = export_for_editor(&document).unwrap();
        assert!(markdown.ends_with("\n\n<!-- Enter prompt content here. -->"));
        assert_eq!(parse_editor(&markdown).unwrap(), document);

        let with_content = format!("{markdown}\nReview this code.");
        assert_eq!(
            parse_editor(&with_content).unwrap().content,
            "Review this code."
        );

        let before_placeholder = with_content.replace(
            CONTENT_PLACEHOLDER,
            &format!("Review this code.\n{CONTENT_PLACEHOLDER}"),
        );
        assert_eq!(
            parse_editor(&before_placeholder).unwrap().content,
            "Review this code.\nReview this code."
        );
    }

    #[test]
    fn preserves_an_intentional_leading_blank_line() {
        let document = parse("---\nname: test\n---\n\n\nbody").unwrap();
        assert_eq!(document.content, "\nbody");
    }

    #[test]
    fn rejects_unknown_metadata() {
        assert!(parse("---\nname: test\nmodel: example\n---\n\nbody").is_err());
    }

    #[test]
    fn rejects_invalid_exec_commands() {
        assert!(parse("---\nname: test\nexec: '  '\n---\n\nbody").is_err());
        assert!(parse("---\nname: test\nexec: \"codex '\"\n---\n\nbody").is_err());
    }

    #[test]
    fn accepts_blank_optional_metadata() {
        let document = parse(
            "---\nname: test\ndescription: # Optional description\ntags: # Optional list\nexec: # Optional command\n---\n\nbody",
        )
        .unwrap();
        assert_eq!(document.description, None);
        assert!(document.tags.is_empty());
        assert_eq!(document.exec, None);
        assert_eq!(document.content, "body");
    }

    #[test]
    fn accepts_crlf_front_matter() {
        let document = parse("---\r\nname: test\r\n---\r\n\r\nbody\r\n").unwrap();
        assert_eq!(document.content, "body\r\n");
    }

    #[test]
    fn reports_template_errors_at_the_markdown_file_line() {
        let error = parse("---\nname: test\n---\n\nbody\n{{ invalid name }}").unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid template syntax at line 6, column 1: invalid expression `{{ invalid name }}`"
        );
    }
}
