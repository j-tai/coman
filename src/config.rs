use std::collections::HashMap;

use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub src_dir: String,
    pub test_dir: String,
    pub build_dir: String,
    pub timeout: u64,
    pub languages: HashMap<String, Language>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            src_dir: "src".to_string(),
            test_dir: "test".to_string(),
            build_dir: "build".to_string(),
            timeout: 5000,
            languages: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Language {
    pub compile: Vec<String>,
    pub compile_debug: Vec<String>,
    pub run: Vec<String>,
    pub debug: Vec<String>,
}
