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
    /// Print a prompt without rendering it.
    Get(NameArgs),
    /// Render a prompt with variables and piped input.
    Render(RenderArgs),
    /// List prompt names.
    List(ListArgs),
    /// Search prompt names, descriptions, and bodies.
    Search(SearchArgs),
    /// List prompts ordered by most recent use.
    Recent,
    /// Select a prompt name using fzf.
    Pick,
    /// Import a Markdown prompt file.
    Import(ImportArgs),
    /// Export a prompt as Markdown.
    Export(ExportArgs),
    /// Add or remove a prompt from favorites.
    Favorite(FavoriteArgs),
    /// List the saved versions of a prompt.
    History(NameArgs),
    /// Diff two saved prompt versions.
    Diff(DiffArgs),
    /// Check a Markdown prompt without importing it.
    Check(CheckArgs),
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
pub struct RenderArgs {
    #[arg(add = ArgValueCompleter::new(prompt_name_completer))]
    pub name: String,

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
pub struct DiffArgs {
    /// Older version as NAME@VERSION.
    #[arg(add = ArgValueCompleter::new(version_reference_completer))]
    pub old: VersionReference,
    /// Newer version as NAME@VERSION.
    #[arg(add = ArgValueCompleter::new(version_reference_completer))]
    pub new: VersionReference,
}

#[derive(Clone, Debug)]
pub struct VersionReference {
    pub name: String,
    pub version: i64,
}

impl FromStr for VersionReference {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let (name, version) = value
            .rsplit_once('@')
            .ok_or_else(|| "expected NAME@VERSION".to_owned())?;
        crate::prompt::validate_name(name).map_err(|error| error.to_string())?;
        let version = version
            .parse::<i64>()
            .map_err(|_| "version must be a positive integer".to_owned())?;
        if version < 1 {
            return Err("version must be a positive integer".into());
        }
        Ok(Self {
            name: name.to_owned(),
            version,
        })
    }
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Markdown file to check, or - to read stdin.
    pub file: PathBuf,
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

fn version_reference_completer(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return Vec::new();
    };
    let Some(database) = completion_database() else {
        return Vec::new();
    };
    let name_prefix = current.split_once('@').map_or(current, |(name, _)| name);
    let Ok(names) = database.list_prompt_names() else {
        return Vec::new();
    };
    names
        .into_iter()
        .filter(|name| name.starts_with(name_prefix))
        .flat_map(|name| {
            database
                .prompt_history(&name)
                .unwrap_or_default()
                .into_iter()
                .map(move |version| format!("{name}@{}", version.version))
        })
        .filter(|reference| reference.starts_with(current))
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
