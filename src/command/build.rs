use std::fs;
use std::io;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::Program;

use super::eval_command_template;

/// Compile the program.
pub fn recompile(prog: &Program, debug: bool) -> Result<()> {
    let src = prog.source_path();
    let dst = prog.build_path(debug);
    let ext = prog.source_extension();
    if let Some(lang) = prog.language() {
        let cmd = if debug && !lang.compile_debug.is_empty() {
            &lang.compile_debug
        } else {
            &lang.compile
        };
        // Create destination parent directories
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create dir {:?}", parent))?;
        }
        if cmd.is_empty() {
            // Copy src -> dst
            fs::copy(src, dst)
                .with_context(|| format!("failed to copy file {:?} to {:?}", src, dst))?;
            // Set executable
            // let mut perm = fs::metadata(dst)?.permissions();
            // perm.set_mode(perm.mode() | 0o111);
            // fs::set_permissions(dst, perm)?;
        } else {
            // Run compilation command
            let mut cmd = eval_command_template(prog, cmd, debug);
            let stat = cmd
                .status()
                .with_context(|| format!("failed to run command {:?}", cmd))?;
            if !stat.success() {
                bail!("command exited with error status: {:?}", cmd);
            }
        }
        Ok(())
    } else {
        bail!("unknown file extension {:?}", ext);
    }
}

/// Check if the source file needs a recompile, e.g. due to modification.
pub fn is_dirty(prog: &Program, debug: bool) -> bool {
    fn try_check_dirty(dst: &Path, src: &Path) -> io::Result<bool> {
        let dst_time = dst.metadata()?.modified()?;
        let src_time = src.metadata()?.modified()?;
        Ok(dst_time < src_time)
    }

    fn check_dirty(dst: &Path, src: &Path) -> bool {
        try_check_dirty(dst, src).unwrap_or(true)
    }

    check_dirty(prog.build_path(debug), prog.source_path())
        || check_dirty(prog.build_path(debug), prog.repository().config_path())
}

/// Compile the program if it has not already been compiled. If it does not need
/// to be compiled, no action is performed and `Ok` is returned.
pub fn compile(prog: &Program, debug: bool) -> Result<()> {
    if is_dirty(prog, debug) {
        recompile(prog, debug)
    } else {
        Ok(())
    }
}
