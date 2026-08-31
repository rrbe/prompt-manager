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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_exec"
    )]
    exec: Option<String>,
}

fn deserialize_optional_exec<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(Some)
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
    let yaml = serde_yaml::to_string(&metadata)?;
    Ok(format!("---\n{yaml}---\n\n{}", document.content))
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
        assert!(parse("---\nname: test\nexec:\n---\n\nbody").is_err());
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
