use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{env, fmt};

use anyhow::{bail, Context, Result};
use if_chain::if_chain;
use walkdir::WalkDir;

use crate::Config;
use crate::Language;

/// Finds the root directory of the contests repository.
///
/// This function searches the cwd's parent directories until it finds one
/// containing `Coman.toml`. This returns `None` if no root directory was found
/// or the current directory cannot be fetched.
pub fn find_root_dir() -> Result<PathBuf> {
    let mut path = env::current_dir().context("failed to get current dir")?;
    loop {
        path.push("Coman.toml");
        if path.exists() {
            path.pop();
            return Ok(path);
        }
        path.pop();
        if !path.pop() {
            bail!("cannot find repository root; make sure you have a Coman.toml")
        }
    }
}

/// A struct representing a contest repository.
///
/// This struct is immutable.
#[derive(Clone)]
pub struct Repository {
    config: Config,
    config_path: PathBuf,
    root: PathBuf,
    src: PathBuf,
    test: PathBuf,
    build: PathBuf,
    build_release: PathBuf,
    build_debug: PathBuf,
}

impl Repository {
    /// Create a new `Repository`.
    pub fn new<P: Into<PathBuf>>(root: P, config: Config) -> Repository {
        let root = root.into();
        let mut config_path = root.clone();
        config_path.push("Coman.toml");
        let mut src = root.clone();
        src.push(&config.src_dir);
        let mut test = root.clone();
        test.push(&config.test_dir);
        let mut build = root.clone();
        build.push(&config.build_dir);
        let mut build_release = build.clone();
        build_release.push("release");
        let mut build_debug = build.clone();
        build_debug.push("debug");
        Repository {
            config,
            config_path,
            root,
            src,
            test,
            build,
            build_release,
            build_debug,
        }
    }

    /// Create a new `Repository`, reading the configuration files
    /// from the 'Coman.toml' file under the specified path.
    pub fn read(root: impl Into<PathBuf>) -> Result<Repository> {
        let mut root = root.into();
        root.push("Coman.toml");
        let config = match File::open(&root) {
            Ok(mut f) => {
                let mut s = String::new();
                f.read_to_string(&mut s)
                    .context("failed to read Coman.toml")?;
                toml::from_str::<Config>(&s).context("failed to parse Coman.toml")?
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => Config::default(),
            Err(e) => return Err(e).context("failed to read Coman.toml")?,
        };
        root.pop();
        Ok(Repository::new(root, config))
    }

    /// Get the repository's configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the path to the repository's configuration.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get the repository's root directory path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the repository's source directory path.
    pub fn source_path(&self) -> &Path {
        &self.src
    }

    /// Get the repository's test directory path.
    pub fn test_path(&self) -> &Path {
        &self.test
    }

    /// Get the repository's build directory path.
    pub fn build_path(&self) -> &Path {
        &self.build
    }

    /// Get the repository's release build directory path.
    pub fn build_release_path(&self) -> &Path {
        &self.build_release
    }

    /// Get the repository's debug build directory path.
    pub fn build_debug_path(&self) -> &Path {
        &self.build_debug
    }

    /// Get a `Program` from the path to its source code. Returns
    /// `None` if the path is outside of the source directory or if it
    /// does not exist.
    pub fn get_program<P: AsRef<Path>>(&self, path: P) -> Result<Program> {
        let path = path.as_ref();
        let path = match path.canonicalize() {
            Ok(p) => p,
            Err(ref e) if e.kind() == ErrorKind::NotFound => bail!("file not found: {:?}", path),
            Err(e) => {
                return Err(e).with_context(|| format!("failed to canonicalize path {:?}", path));
            }
        };
        if !path.is_file() {
            bail!("not a file: {:?}", path);
        }
        let path = path
            .strip_prefix(self.source_path())
            .with_context(|| format!("file is not inside repository: {:?}", path))?;

        let mut src = self.source_path().to_path_buf();
        src.push(&path);
        let mut test = self.test_path().to_path_buf();
        test.push(&path);
        while test.extension().is_some() {
            test.set_extension("");
        }
        let mut build_release = self.build_release_path().to_path_buf();
        build_release.push(&path);
        let mut build_debug = self.build_debug_path().to_path_buf();
        build_debug.push(&path);

        Ok(Program {
            repo: self,
            path: path.to_path_buf(),
            src,
            test,
            build_release,
            build_debug,
        })
    }

    /// Get the `Program` that was most recently modified. Returns
    /// `None` if no program could be found.
    pub fn find_recent_program(&self) -> Result<Program> {
        let mut best_time = SystemTime::UNIX_EPOCH;
        let mut best_prog = None;
        for ent in WalkDir::new(self.source_path()).into_iter().flatten() {
            if_chain! {
                if ent.file_type().is_file();
                if let Some(ext) = ent.path().extension().and_then(|s| s.to_str());
                if self.config.languages.contains_key(ext);
                if let Ok(meta) = ent.metadata();
                if let Ok(modified) = meta.modified();
                if modified > best_time;
                then {
                    best_time = modified;
                    best_prog = Some(ent.into_path());
                }
            }
        }

        if let Some(path) = best_prog {
            self.get_program(path)
        } else {
            bail!("no solutions found");
        }
    }
}

/// A struct representing a program in a repository.
///
/// This struct is immutable.
pub struct Program<'a> {
    repo: &'a Repository,
    path: PathBuf,
    src: PathBuf,
    test: PathBuf,
    build_release: PathBuf,
    build_debug: PathBuf,
}

impl Program<'_> {
    /// Get the `Repository` in which this program is contained.
    pub fn repository(&self) -> &Repository {
        self.repo
    }

    /// Get the name of the program.
    pub fn name(&self) -> &str {
        self.path.to_str().unwrap()
    }

    /// Get the path to the program's source file.
    pub fn source_path(&self) -> &Path {
        &self.src
    }

    /// Get the source file's extension.
    ///
    /// Returns an empty string if the file has no extension.
    pub fn source_extension(&self) -> &str {
        self.path.extension().and_then(|s| s.to_str()).unwrap_or("")
    }

    /// Get the path to the program's test directory.
    pub fn test_path(&self) -> &Path {
        &self.test
    }

    /// Get the path to the program's release build location.
    pub fn build_release_path(&self) -> &Path {
        &self.build_release
    }

    /// Get the path to the program's debug build location.
    pub fn build_debug_path(&self) -> &Path {
        &self.build_debug
    }

    /// Get the path to the program's build location.
    pub fn build_path(&self, debug: bool) -> &Path {
        if debug {
            self.build_debug_path()
        } else {
            self.build_release_path()
        }
    }

    /// Get the language that this program is written in.
    pub fn language(&self) -> Option<&Language> {
        self.repo.config().languages.get(self.source_extension())
    }
}

impl fmt::Display for Program<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
