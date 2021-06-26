use std::fs::File;
use std::io::Write;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::Repository;

/// Write a CMakeLists.txt file to the given writer.
pub fn write_cmake_to(repo: &Repository, mut w: impl Write) -> Result<()> {
    writeln!(w, "cmake_minimum_required(VERSION 3.9)")?;
    writeln!(
        w,
        "project({:?})",
        repo.root().file_name().unwrap().to_string_lossy(),
    )?;
    writeln!(w, "set(CMAKE_CXX_STANDARD 11)")?;
    // Add all the programs
    for ent in WalkDir::new(repo.source_path()) {
        if let Ok(ent) = ent {
            if ent.file_type().is_file() {
                let path = ent.path().strip_prefix(repo.source_path()).unwrap();
                let src_path = ent.path().strip_prefix(repo.root()).unwrap();
                let bin_path = path.to_string_lossy().replace('/', ".");
                match ent.path().extension().unwrap().to_str() {
                    Some("cpp") => {
                        writeln!(w, "add_executable({:?} {:?})", bin_path, src_path.display())?;
                        writeln!(
                            w,
                            "set_target_properties({:?} PROPERTIES LINKER_LANGUAGE CXX)",
                            bin_path,
                        )?;
                    }
                    Some("c") => {
                        writeln!(w, "add_executable({:?} {:?})", bin_path, src_path.display())?;
                        writeln!(
                            w,
                            "set_target_properties({:?} PROPERTIES LINKER_LANGUAGE C)",
                            bin_path,
                        )?;
                    }
                    _ => (),
                }
            }
        }
    }
    Ok(())
}

/// Write or update the CMakeLists.txt file.
pub fn write_cmake(repo: &Repository) -> Result<()> {
    let filename = repo.root().join("CMakeLists.txt");
    let f =
        File::create(&filename).with_context(|| format!("failed to create file {:?}", filename))?;
    write_cmake_to(repo, f)
}
