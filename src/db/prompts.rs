use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::error::{Error, Result};

use super::{Database, history, now_timestamp, tags};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prompt {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub exec: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub use_count: i64,
    pub favorite: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptInput {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub exec: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptListEntry {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub exec: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_used_at: Option<i64>,
    pub use_count: i64,
    pub favorite: bool,
}

impl Database {
    pub fn prompt_exists(&self, name: &str) -> Result<bool> {
        Ok(prompt_id(&self.connection, name)?.is_some())
    }

    pub fn create_prompt(&mut self, input: &PromptInput) -> Result<()> {
        let transaction = self.connection.transaction()?;
        if prompt_id(&transaction, &input.name)?.is_some() {
            return Err(Error::PromptAlreadyExists(input.name.clone()));
        }

        let now = now_timestamp();
        transaction.execute("INSERT INTO prompt_id_sequence DEFAULT VALUES", [])?;
        let id = transaction.last_insert_rowid();
        transaction.execute("DELETE FROM prompt_id_sequence WHERE id = ?1", [id])?;
        transaction.execute(
            "INSERT INTO prompts(id, name, description, content, exec, created_at, updated_at)\n\
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                id,
                input.name,
                input.description,
                input.content,
                input.exec,
                now
            ],
        )?;
        tags::replace(&transaction, id, &input.tags)?;
        history::record_version(&transaction, id, input, now)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn get_prompt(&mut self, name: &str) -> Result<Prompt> {
        let transaction = self.connection.transaction()?;
        let prompt = query_prompt(&transaction, name)?
            .ok_or_else(|| Error::PromptNotFound(name.to_owned()))?;
        transaction.commit()?;
        Ok(prompt)
    }

    pub fn mark_prompt_used(&mut self, name: &str) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE prompts\n\
             SET last_used_at = ?1, use_count = use_count + 1\n\
             WHERE name = ?2",
            params![now_timestamp(), name],
        )?;
        if changed == 0 {
            return Err(Error::PromptNotFound(name.to_owned()));
        }
        Ok(())
    }

    pub fn update_prompt(&mut self, original_name: &str, input: &PromptInput) -> Result<()> {
        let transaction = self.connection.transaction()?;
        let Some(current) = query_prompt(&transaction, original_name)? else {
            return Err(Error::PromptNotFound(original_name.to_owned()));
        };
        let id = current.id;
        if input.name != original_name && prompt_id(&transaction, &input.name)?.is_some() {
            return Err(Error::PromptAlreadyExists(input.name.clone()));
        }

        if current.name == input.name
            && current.description == input.description
            && current.content == input.content
            && current.tags == input.tags
            && current.exec == input.exec
        {
            transaction.commit()?;
            return Ok(());
        }

        let now = now_timestamp();
        transaction.execute(
            "UPDATE prompts\n\
             SET name = ?1, description = ?2, content = ?3, exec = ?4, updated_at = ?5\n\
             WHERE id = ?6",
            params![
                input.name,
                input.description,
                input.content,
                input.exec,
                now,
                id
            ],
        )?;
        tags::replace(&transaction, id, &input.tags)?;
        history::record_version(&transaction, id, input, now)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_prompt(&mut self, name: &str) -> Result<()> {
        let transaction = self.connection.transaction()?;
        let changed = transaction.execute("DELETE FROM prompts WHERE name = ?1", [name])?;
        if changed == 0 {
            return Err(Error::PromptNotFound(name.to_owned()));
        }
        tags::cleanup_orphans(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_prompt_names(&self) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT name FROM prompts ORDER BY name")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn prompt_name_by_id(&self, id: i64) -> Result<String> {
        self.connection
            .query_row("SELECT name FROM prompts WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or_else(|| Error::PromptNotFound(id.to_string()))
    }

    pub fn set_prompt_favorite(&self, name: &str, favorite: bool) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE prompts SET favorite = ?1 WHERE name = ?2",
            params![favorite, name],
        )?;
        if changed == 0 {
            return Err(Error::PromptNotFound(name.to_owned()));
        }
        Ok(())
    }

    pub fn list_prompts_filtered(
        &self,
        group: Option<&str>,
        tag: Option<&str>,
        favorite_only: bool,
    ) -> Result<Vec<PromptListEntry>> {
        let mut statement = self.connection.prepare(
            "SELECT prompts.id, prompts.name, prompts.description, prompts.exec,\n\
                    prompts.created_at, prompts.updated_at, prompts.last_used_at,\n\
                    prompts.use_count, prompts.favorite, tags.name\n\
             FROM prompts\n\
             LEFT JOIN prompt_tags ON prompt_tags.prompt_id = prompts.id\n\
             LEFT JOIN tags ON tags.id = prompt_tags.tag_id\n\
             WHERE (?1 IS NULL OR substr(prompts.name, 1, length(?1)) = ?1)\n\
             AND (?2 IS NULL OR EXISTS (\n\
                 SELECT 1\n\
                 FROM prompt_tags\n\
                 JOIN tags ON tags.id = prompt_tags.tag_id\n\
                 WHERE prompt_tags.prompt_id = prompts.id AND tags.name = ?2\n\
             ))\n\
             AND (?3 = 0 OR prompts.favorite = 1)\n\
             ORDER BY prompts.name, tags.name",
        )?;
        let mut rows = statement.query(params![group, tag, favorite_only])?;
        let mut prompts = Vec::<PromptListEntry>::new();
        while let Some(row) = rows.next()? {
            let id = row.get(0)?;
            if prompts.last().is_none_or(|prompt| prompt.id != id) {
                prompts.push(PromptListEntry {
                    id,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    tags: Vec::new(),
                    exec: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    last_used_at: row.get(6)?,
                    use_count: row.get(7)?,
                    favorite: row.get(8)?,
                });
            }
            if let Some(tag) = row.get::<_, Option<String>>(9)? {
                prompts
                    .last_mut()
                    .expect("the current prompt was added before its tag")
                    .tags
                    .push(tag);
            }
        }
        Ok(prompts)
    }
}

fn prompt_id(connection: &Connection, name: &str) -> Result<Option<i64>> {
    connection
        .query_row("SELECT id FROM prompts WHERE name = ?1", [name], |row| {
            row.get(0)
        })
        .optional()
        .map_err(Into::into)
}

fn query_prompt(transaction: &Transaction<'_>, name: &str) -> Result<Option<Prompt>> {
    let row = transaction
        .query_row(
            "SELECT id, name, description, content, exec, created_at, updated_at, last_used_at, use_count, favorite\n\
             FROM prompts WHERE name = ?1",
            [name],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, bool>(9)?,
                ))
            },
        )
        .optional()?;

    let Some((
        id,
        name,
        description,
        content,
        exec,
        created_at,
        updated_at,
        last_used_at,
        use_count,
        favorite,
    )) = row
    else {
        return Ok(None);
    };
    Ok(Some(Prompt {
        id,
        name,
        description,
        content,
        tags: tags::get(transaction, id)?,
        exec,
        created_at,
        updated_at,
        last_used_at,
        use_count,
        favorite,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, content: &str, tags: &[&str]) -> PromptInput {
        PromptInput {
            name: name.into(),
            description: None,
            content: content.into(),
            tags: tags.iter().map(|tag| (*tag).into()).collect(),
            exec: None,
        }
    }

    #[test]
    fn creates_updates_renames_and_deletes_prompt_atomically() {
        let mut database = Database::in_memory().unwrap();
        database
            .create_prompt(&input("first", "old", &["coding", "review"]))
            .unwrap();
        assert_eq!(database.get_prompt("first").unwrap().tags.len(), 2);

        database
            .update_prompt("first", &input("renamed", "new", &["updated"]))
            .unwrap();
        assert!(matches!(
            database.get_prompt("first"),
            Err(Error::PromptNotFound(_))
        ));
        let prompt = database.get_prompt("renamed").unwrap();
        assert_eq!(prompt.content, "new");
        assert_eq!(prompt.tags, vec!["updated"]);

        database.delete_prompt("renamed").unwrap();
        assert!(database.list_prompt_names().unwrap().is_empty());
    }

    #[test]
    fn failed_rename_leaves_original_unchanged() {
        let mut database = Database::in_memory().unwrap();
        database.create_prompt(&input("one", "1", &[])).unwrap();
        database.create_prompt(&input("two", "2", &[])).unwrap();
        assert!(matches!(
            database.update_prompt("one", &input("two", "changed", &[])),
            Err(Error::PromptAlreadyExists(_))
        ));
        assert_eq!(database.get_prompt("one").unwrap().content, "1");
    }

    #[test]
    fn tracks_usage() {
        let mut database = Database::in_memory().unwrap();
        database.create_prompt(&input("used", "body", &[])).unwrap();
        database.mark_prompt_used("used").unwrap();
        database.mark_prompt_used("used").unwrap();
        let prompt = database.get_prompt("used").unwrap();
        assert_eq!(prompt.use_count, 2);
        assert!(prompt.last_used_at.is_some());
    }

    #[test]
    fn does_not_reuse_deleted_prompt_ids() {
        let mut database = Database::in_memory().unwrap();
        database.create_prompt(&input("one", "1", &[])).unwrap();
        database.create_prompt(&input("two", "2", &[])).unwrap();
        database.delete_prompt("two").unwrap();
        database.create_prompt(&input("three", "3", &[])).unwrap();

        assert_eq!(database.get_prompt("one").unwrap().id, 1);
        assert_eq!(database.get_prompt("three").unwrap().id, 3);
    }

    #[test]
    fn filters_prompts_by_tag() {
        let mut database = Database::in_memory().unwrap();
        database
            .create_prompt(&input("alpha", "a", &["coding", "review"]))
            .unwrap();
        database
            .create_prompt(&input("beta", "b", &["writing"]))
            .unwrap();
        let prompts = database
            .list_prompts_filtered(None, Some("coding"), false)
            .unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "alpha");
        assert_eq!(prompts[0].tags, ["coding", "review"]);
    }

    #[test]
    fn records_versions_only_for_content_or_metadata_changes() {
        let mut database = Database::in_memory().unwrap();
        database
            .create_prompt(&input("versioned", "one", &["first"]))
            .unwrap();
        assert_eq!(database.prompt_history("versioned").unwrap().len(), 1);

        database
            .update_prompt("versioned", &input("versioned", "two", &["second"]))
            .unwrap();
        let history = database.prompt_history("versioned").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 2);
        let version = database.prompt_version("versioned", 2).unwrap();
        assert_eq!(version.content, "two");
        assert_eq!(version.tags, vec!["second"]);

        database
            .update_prompt("versioned", &input("versioned", "two", &["second"]))
            .unwrap();
        database.set_prompt_favorite("versioned", true).unwrap();
        assert_eq!(database.prompt_history("versioned").unwrap().len(), 2);
        assert!(database.get_prompt("versioned").unwrap().favorite);
        assert_eq!(
            database
                .list_prompts_filtered(None, None, true)
                .unwrap()
                .into_iter()
                .map(|prompt| prompt.name)
                .collect::<Vec<_>>(),
            vec!["versioned"]
        );
    }
}
