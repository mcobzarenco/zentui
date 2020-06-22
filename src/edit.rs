use anyhow::Result;
use std::{fs, io::Write};
use tempfile::{self, NamedTempFile};

pub fn edit(text: &str) -> Result<String> {
    let mut file = tempfile::Builder::new().suffix(".md").tempfile()?;
    file.write(text.as_bytes())?;
    let path = file.into_temp_path();
    ::edit::edit_file(&path)?;
    let edited = fs::read(&path)?;
    path.close()?;
    Ok(String::from_utf8(edited)?)
}
