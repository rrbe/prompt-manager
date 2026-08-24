pub mod markdown;
pub mod template;

use std::collections::BTreeSet;

use crate::error::{Error, Result};

pub fn validate_name(name: &str) -> Result<()> {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(Error::Message("prompt name must not be empty".into()));
    };

    if !first.is_ascii_alphanumeric()
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return Err(Error::Message(format!("invalid prompt name: {name}")));
    }
    Ok(())
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
        for name in ["code-review", "typescript.review", "debug_log", "A1"] {
            validate_name(name).unwrap();
        }
        for name in ["", "has space", "/path", "_starts-wrong", "shell;code"] {
            assert!(validate_name(name).is_err(), "{name} should be invalid");
        }
    }

    #[test]
    fn normalizes_and_sorts_tags() {
        assert_eq!(
            normalize_tags(vec![" review ".into(), "coding".into(), "review".into()]).unwrap(),
            vec!["coding", "review"]
        );
    }
}
