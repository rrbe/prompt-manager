use rusqlite::params;

use crate::error::{Error, Result};

use super::Database;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    pub id: i64,
    pub name: String,
    pub description: String,
}

impl Database {
    pub fn search_prompts(&self, query: &str) -> Result<Vec<SearchResult>> {
        let query = literal_fts_query(query)?;
        let mut statement = self.connection.prepare(
            "SELECT prompts.id, prompts.name, COALESCE(prompts.description, '')\n\
             FROM prompts_fts\n\
             JOIN prompts ON prompts.id = prompts_fts.rowid\n\
             WHERE prompts_fts MATCH ?1\n\
             ORDER BY bm25(prompts_fts), prompts.name",
        )?;
        let rows = statement.query_map(params![query], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

fn literal_fts_query(query: &str) -> Result<String> {
    let terms: Vec<_> = query
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return Err(Error::Message("search query must not be empty".into()));
    }
    Ok(terms.join(" AND "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::PromptInput;

    fn create(database: &mut Database, name: &str, description: Option<&str>, content: &str) {
        database
            .create_prompt(&PromptInput {
                name: name.into(),
                description: description.map(Into::into),
                content: content.into(),
                tags: vec![],
            })
            .unwrap();
    }

    #[test]
    fn fts_tracks_insert_update_and_delete() {
        let mut database = Database::in_memory().unwrap();
        create(
            &mut database,
            "mongo-review",
            Some("database check"),
            "old keyword",
        );
        assert_eq!(
            database.search_prompts("mongo").unwrap()[0].name,
            "mongo-review"
        );
        assert_eq!(database.search_prompts("database").unwrap().len(), 1);
        assert_eq!(database.search_prompts("keyword").unwrap().len(), 1);

        database
            .update_prompt(
                "mongo-review",
                &PromptInput {
                    name: "sql-review".into(),
                    description: Some("relational".into()),
                    content: "new token".into(),
                    tags: vec![],
                },
            )
            .unwrap();
        assert!(database.search_prompts("mongo").unwrap().is_empty());
        assert_eq!(database.search_prompts("relational").unwrap().len(), 1);

        database.delete_prompt("sql-review").unwrap();
        assert!(database.search_prompts("relational").unwrap().is_empty());
    }

    #[test]
    fn treats_fts_syntax_as_literal_text() {
        assert_eq!(
            literal_fts_query("one OR two").unwrap(),
            "\"one\" AND \"OR\" AND \"two\""
        );
        assert!(literal_fts_query("  ").is_err());
    }
}
