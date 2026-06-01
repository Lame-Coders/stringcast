use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigPathError {
    ProjectDirsUnavailable,
}

pub fn config_file_path() -> Result<PathBuf, ConfigPathError> {
    let dirs = directories::ProjectDirs::from("", "", "Stringcast")
        .ok_or(ConfigPathError::ProjectDirsUnavailable)?;
    Ok(dirs.config_dir().join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_ends_with_config_toml() {
        let path = config_file_path().unwrap();

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("config.toml")
        );
    }
}
