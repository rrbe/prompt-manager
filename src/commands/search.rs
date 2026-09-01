use crate::{cli::SearchArgs, db::Database, error::Result};

use super::{clean_inline, write_stdout};

pub fn run(arguments: SearchArgs, database: &Database) -> Result<()> {
    let results = database.search_prompts(&arguments.query)?;
    if results.is_empty() {
        return Ok(());
    }

    let lines = results
        .into_iter()
        .map(|result| {
            format!(
                "{}\t{}\t{}",
                result.id,
                result.name,
                clean_inline(&result.description)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    write_stdout(&format!("{lines}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tsv_control_characters() {
        assert_eq!(clean_inline("one\ttwo\nthree\r"), "one two three ");
    }
}
