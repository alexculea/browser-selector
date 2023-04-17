use std::path::PathBuf;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crate::os::macos::sys_browsers::Browser;

#[derive(Default, Debug, Clone)]
pub struct ActiveWindowInfo {
    pub window_name: Option<String>,
    pub exe_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum BinaryType {
    Bits32,
    Bits64,
    None,
}

impl Default for BinaryType {
    fn default() -> BinaryType {
        BinaryType::None
    }
}

impl std::fmt::Display for BinaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinaryType::Bits32 => "32 bits",
                BinaryType::Bits64 => "64 bits",
                _ => "",
            }
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct VersionInfo {
    pub company_name: String,
    pub file_description: String,
    pub product_version: String,
    pub product_name: String,
    pub binary_type: BinaryType,
}

impl Browser { 
    pub fn get_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.exe_path.hash(&mut hasher);
        hasher.finish().to_string()
    }
}

pub fn spawn_browser_process(exe_path: &String, args: Vec<String>, url: &str) {
    let mut command_arguments = args;
    command_arguments.push(String::from(url));

    std::process::Command::new(exe_path)
        .args(command_arguments)
        .spawn()
        .expect(
            format!("Couldn't run browser program at {}", exe_path)
                .to_owned()
                .as_str(),
        );
}