use std::io::{self, IsTerminal, Read};

use crate::error::{Error, Result};

pub fn read_piped_input() -> Result<String> {
    let stdin = io::stdin();
    read_input(stdin.is_terminal(), stdin.lock())
}

fn read_input(is_terminal: bool, reader: impl Read) -> Result<String> {
    if is_terminal {
        return Err(Error::MissingVariable("input".into()));
    }

    read_utf8(reader)
}

fn read_utf8(mut reader: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| Error::Message("stdin is not valid UTF-8".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty_piped_input() {
        assert_eq!(read_utf8(&b""[..]).unwrap(), "");
    }

    #[test]
    fn rejects_non_utf8_input() {
        assert!(read_utf8(&[0xff][..]).is_err());
    }

    #[test]
    fn refuses_to_read_interactive_input() {
        let error = read_input(true, &b"ignored"[..]).unwrap_err();
        assert!(matches!(error, Error::MissingVariable(name) if name == "input"));
    }
}
