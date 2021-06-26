use std::fs::{self, File};
use std::io::{ErrorKind, Write};

use anyhow::{Context, Result};

const DEFAULT_COMAN_TOML: &'static str = include_str!("default_coman.toml");

pub fn init() -> Result<()> {
    // Create Coman.toml
    let meta = fs::metadata("Coman.toml");
    if let Err(e) = meta {
        if e.kind() == ErrorKind::NotFound {
            // Since the file does not exist, we should create it
            let mut file = File::create("Coman.toml").context("failed to create Coman.toml")?;
            file.write_all(DEFAULT_COMAN_TOML.as_bytes())
                .context("failed to write to Coman.toml")?;
        }
    } else {
        meta.context("failed to stat Coman.toml")?;
    }

    // Create dirs
    for dir in ["src", "test"] {
        fs::create_dir_all(dir).with_context(|| format!("failed to create dir {:?}", dir))?;
    }

    Ok(())
}
