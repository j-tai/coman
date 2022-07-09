use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::{Program, Repository};

/// Clean all compiled binaries from the repository.
pub fn clean_all(repo: &Repository) -> Result<()> {
    match fs::remove_dir_all(repo.build_path()) {
        Ok(()) => Ok(()),
        Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to delete dir {:?}", repo.build_path())),
    }
}

/// Clean the program's binaries. This deletes the debug and
/// release binaries if they exist.
pub fn clean(prog: &Program) -> Result<()> {
    fn try_delete_file(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(ref e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).with_context(|| format!("failed to delete file {:?}", path)),
        }
    }
    try_delete_file(prog.build_debug_path())?;
    try_delete_file(prog.build_release_path())?;
    Ok(())
}
