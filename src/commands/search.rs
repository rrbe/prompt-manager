use crate::{cli::SearchArgs, db::Database, error::Result};

use super::write_stdout;

pub fn run(arguments: SearchArgs, database: &Database) -> Result<()> {
    let results = database.search_prompts(&arguments.query)?;
    if results.is_empty() {
        return Ok(());
    }

    let lines = results
        .into_iter()
        .map(|result| {
            if arguments.name_only {
                result.name
            } else {
                format!(
                    "{}\t{}\t{}",
                    result.id,
                    result.name,
                    clean_description(&result.description)
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_stdout(&format!("{lines}\n"))
}

fn clean_description(description: &str) -> String {
    description
        .chars()
        .map(|character| match character {
            '\t' | '\r' | '\n' => ' ',
            _ => character,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tsv_control_characters() {
        assert_eq!(clean_description("one\ttwo\nthree\r"), "one two three ");
    }
}
