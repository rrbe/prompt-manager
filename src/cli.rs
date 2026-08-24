use std::{ffi::OsStr, path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use crate::prompt::template::is_valid_variable_name;

#[derive(Debug, Parser)]
#[command(name = "pm", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a prompt using an external editor.
    Add(AddArgs),
    /// Edit an existing prompt using an external editor.
    Edit(NameArgs),
    /// Remove a prompt.
    Rm(RemoveArgs),
    /// Get a prompt with variables and piped input.
    Get(GetArgs),
    /// List prompt IDs and names.
    List(ListArgs),
    /// Search prompt names, descriptions, and bodies.
    Search(SearchArgs),
    /// Import a Markdown prompt file.
    Import(ImportArgs),
    /// Export a prompt as Markdown.
    Export(ExportArgs),
    /// Add or remove a prompt from favorites.
    Favorite(FavoriteArgs),
    /// Inspect the saved versions of a prompt.
    History(HistoryArgs),
    /// Generate shell completions.
    Completions(CompletionsArgs),
}

#[derive(Debug, Args)]
pub struct NameArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    /// Remove without confirmation.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Prompt name; required unless --id or --pick is used.
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    #[arg(
        required_unless_present_any = ["id", "pick"],
        conflicts_with_all = ["id", "pick"]
    )]
    pub name: Option<String>,

    /// Select a prompt by ID instead of providing NAME.
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..), conflicts_with = "pick")]
    pub id: Option<i64>,

    /// Select a prompt using fzf instead of providing NAME.
    #[arg(long)]
    pub pick: bool,

    /// Set a variable as KEY=VALUE.
    #[arg(short = 'v', long = "var", value_name = "KEY=VALUE")]
    pub variables: Vec<VariableAssignment>,

    /// Read a variable from a UTF-8 file as KEY=PATH.
    #[arg(long = "file", value_name = "KEY=PATH")]
    pub files: Vec<FileAssignment>,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    pub query: String,

    /// Print only prompt names, one per line.
    #[arg(long)]
    pub name_only: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// List only prompts with this tag.
    #[arg(long)]
    pub tag: Option<String>,

    /// List only favorite prompts.
    #[arg(long)]
    pub favorite: bool,

    /// Include updated and last-used timestamps as TSV columns.
    #[arg(short = 'l', long)]
    pub long: bool,

    /// Sort the result.
    #[arg(long, value_enum, default_value_t = ListSort::Name)]
    pub sort: ListSort,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ListSort {
    #[default]
    Name,
    Updated,
    Used,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Prompt name, or output directory when using --all.
    pub target: PathBuf,

    /// Export every prompt to the target directory.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct FavoriteArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    /// Remove the prompt from favorites.
    #[arg(long)]
    pub remove: bool,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

    #[command(subcommand)]
    pub command: Option<HistoryCommand>,
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    /// Diff two saved versions of this prompt.
    Diff(HistoryDiffArgs),
}

#[derive(Debug, Args)]
pub struct HistoryDiffArgs {
    /// Older version number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    pub old: i64,

    /// Newer version number.
    #[arg(value_parser = clap::value_parser!(i64).range(1..))]
    pub new: i64,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: CompletionShell,

    /// Generate a completion script that reads Prompt names at completion time.
    #[arg(long)]
    pub dynamic: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

impl CompletionShell {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VariableAssignment {
    pub key: String,
    pub value: String,
}

impl FromStr for VariableAssignment {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (key, value) = split_assignment(value)?;
        Ok(Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct FileAssignment {
    pub key: String,
    pub path: PathBuf,
}

impl FromStr for FileAssignment {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (key, path) = split_assignment(value)?;
        if path.is_empty() {
            return Err("file path must not be empty".into());
        }
        Ok(Self {
            key: key.to_owned(),
            path: PathBuf::from(path),
        })
    }
}

fn split_assignment(value: &str) -> std::result::Result<(&str, &str), String> {
    let (key, value) = value
        .split_once('=')
        .ok_or_else(|| "expected KEY=VALUE".to_owned())?;
    if !is_valid_variable_name(key) {
        return Err(format!("invalid variable name: {key}"));
    }
    Ok((key, value))
}

fn prompt_name_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let Some(database) = completion_database() else {
        return Vec::new();
    };
    let Ok(names) = database.list_prompt_names() else {
        return Vec::new();
    };
    names
        .into_iter()
        .filter(|name| name.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}

fn completion_database() -> Option<crate::db::Database> {
    let path = crate::paths::database_path().ok()?;
    if !path.is_file() {
        return None;
    }
    crate::db::Database::open_read_only(&path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variable_values_containing_equals() {
        let value: VariableAssignment = "query=a=b".parse().unwrap();
        assert_eq!(value.key, "query");
        assert_eq!(value.value, "a=b");
    }

    #[test]
    fn rejects_invalid_assignment() {
        assert!("not-an-assignment".parse::<VariableAssignment>().is_err());
        assert!("1key=value".parse::<VariableAssignment>().is_err());
    }
}
