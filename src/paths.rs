use std::{env, path::PathBuf};

use crate::error::{Error, Result};

pub fn database_path() -> Result<PathBuf> {
    let data_home = match env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => {
            let home = env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    Error::Message(
                        "cannot determine data directory: XDG_DATA_HOME and HOME are unset".into(),
                    )
                })?;
            PathBuf::from(home).join(".local/share")
        }
    };

    Ok(data_home.join("pm/pm.db"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_pm_database_to_a_data_home() {
        let path = PathBuf::from("/tmp/data").join("pm/pm.db");
        assert_eq!(path, PathBuf::from("/tmp/data/pm/pm.db"));
    }
}
