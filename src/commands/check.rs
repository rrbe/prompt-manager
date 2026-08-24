use std::{fs, io::Read, path::Path};

use crate::{
    cli::CheckArgs,
    error::{Error, Result},
    prompt::markdown,
};

pub fn run(arguments: CheckArgs) -> Result<()> {
    let source = if arguments.file == Path::new("-") {
        read_stdin()?
    } else {
        let bytes = fs::read(&arguments.file).map_err(|source| Error::ReadFile {
            path: arguments.file.clone(),
            source,
        })?;
        String::from_utf8(bytes).map_err(|_| {
            Error::Message(format!(
                "Markdown prompt is not valid UTF-8: {}",
                arguments.file.display()
            ))
        })?
    };

    markdown::parse(&source)?;
    Ok(())
}

fn read_stdin() -> Result<String> {
    let mut bytes = Vec::new();
    std::io::stdin().lock().read_to_end(&mut bytes)?;
    String::from_utf8(bytes)
        .map_err(|_| Error::Message("Markdown prompt from stdin is not valid UTF-8".into()))
}
