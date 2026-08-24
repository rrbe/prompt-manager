pub mod cli;
pub mod commands;
pub mod db;
pub mod editor;
pub mod error;
pub mod paths;
pub mod prompt;
pub mod stdin;

use clap::{CommandFactory, Parser};

use crate::{
    cli::{Cli, Command},
    db::Database,
    error::Result,
};

pub fn run() -> Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command)
        .bin("pm")
        .complete();
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Check(arguments) => commands::check(arguments),
        Command::Completions(arguments) => commands::completions(arguments),
        command => {
            let database_path = paths::database_path()?;
            let mut database = Database::open(&database_path)?;
            commands::execute(command, &mut database)
        }
    }
}
