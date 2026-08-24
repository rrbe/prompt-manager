use rusqlite::{OptionalExtension, Transaction, params};

use crate::error::{Error, Result};

use super::{Database, PromptInput};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptVersionSummary {
    pub version: i64,
    pub name: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptVersion {
    pub version: i64,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: i64,
}

impl Database {
    pub fn prompt_history(&self, name: &str) -> Result<Vec<PromptVersionSummary>> {
        let prompt_id = current_prompt_id(self, name)?;
        let mut statement = self.connection.prepare(
            "SELECT version, name, created_at\n\
             FROM prompt_versions\n\
             WHERE prompt_id = ?1\n\
             ORDER BY version DESC",
        )?;
        let rows = statement.query_map([prompt_id], |row| {
            Ok(PromptVersionSummary {
                version: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn prompt_version(&self, name: &str, version: i64) -> Result<PromptVersion> {
        let prompt_id = current_prompt_id(self, name)?;
        let row = self
            .connection
            .query_row(
                "SELECT id, version, name, description, content, created_at\n\
                 FROM prompt_versions\n\
                 WHERE prompt_id = ?1 AND version = ?2",
                params![prompt_id, version],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| Error::Message(format!("prompt version not found: {name}@{version}")))?;
        let (id, version, name, description, content, created_at) = row;
        let mut statement = self.connection.prepare(
            "SELECT tag FROM prompt_version_tags\n\
             WHERE prompt_version_id = ?1 ORDER BY tag",
        )?;
        let tags = statement
            .query_map([id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(PromptVersion {
            version,
            name,
            description,
            content,
            tags,
            created_at,
        })
    }
}

pub(super) fn record_version(
    transaction: &Transaction<'_>,
    prompt_id: i64,
    input: &PromptInput,
    created_at: i64,
) -> Result<()> {
    let version: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(version), 0) + 1\n\
         FROM prompt_versions WHERE prompt_id = ?1",
        [prompt_id],
        |row| row.get(0),
    )?;
    transaction.execute(
        "INSERT INTO prompt_versions(\n\
            prompt_id, version, name, description, content, created_at\n\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            prompt_id,
            version,
            input.name,
            input.description,
            input.content,
            created_at
        ],
    )?;
    let prompt_version_id = transaction.last_insert_rowid();
    for tag in &input.tags {
        transaction.execute(
            "INSERT INTO prompt_version_tags(prompt_version_id, tag) VALUES (?1, ?2)",
            params![prompt_version_id, tag],
        )?;
    }
    Ok(())
}

fn current_prompt_id(database: &Database, name: &str) -> Result<i64> {
    database
        .connection
        .query_row("SELECT id FROM prompts WHERE name = ?1", [name], |row| {
            row.get(0)
        })
        .optional()?
        .ok_or_else(|| Error::PromptNotFound(name.to_owned()))
}
