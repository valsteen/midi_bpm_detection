#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]

use directories::ProjectDirs;
use lazy_static::lazy_static;
use std::{
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

pub const PROJECT_NAME: &str = "BPM_DETECTION";

lazy_static! {
    pub static ref DATA_FOLDER: Option<PathBuf> = std::env::var(format!("{PROJECT_NAME}_DATA")).ok().map(PathBuf::from);
    pub static ref CONFIG_FOLDER: Option<PathBuf> =
        std::env::var(format!("{PROJECT_NAME}_CONFIG")).ok().map(PathBuf::from);
    pub static ref LOG_ENV: String = format!("{PROJECT_NAME}_LOGLEVEL");
    pub static ref LOG_FILE: String = format!("{PROJECT_NAME}.log");
}

#[must_use]
pub fn get_data_dir() -> PathBuf {
    let directory = if let Some(s) = DATA_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".data")
    };
    directory
}

#[must_use]
pub fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "mbd", env!("CARGO_PKG_NAME"))
}

#[must_use]
pub fn get_config_dir() -> PathBuf {
    let directory = if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    };
    directory
}

pub fn create_build_info() {
    let git_output = std::process::Command::new("git").args(["rev-parse", "--git-dir"]).output().ok();
    let git_dir = git_output.as_ref().and_then(|output| {
        std::str::from_utf8(&output.stdout).ok().and_then(|s| s.strip_suffix('\n').or_else(|| s.strip_suffix("\r\n")))
    });

    // Tell cargo to rebuild if the head or any relevant refs change.
    if let Some(git_dir) = git_dir {
        let git_path = std::path::Path::new(git_dir);
        let refs_path = git_path.join("refs");
        if git_path.join("HEAD").exists() {
            println!("cargo:rerun-if-changed={git_dir}/HEAD");
        }
        if git_path.join("packed-refs").exists() {
            println!("cargo:rerun-if-changed={git_dir}/packed-refs");
        }
        if refs_path.join("heads").exists() {
            println!("cargo:rerun-if-changed={git_dir}/refs/heads");
        }
        if refs_path.join("tags").exists() {
            println!("cargo:rerun-if-changed={git_dir}/refs/tags");
        }
    }

    let git_output =
        std::process::Command::new("git").args(["describe", "--always", "--tags", "--long", "--dirty"]).output().ok();
    let git_info = git_output.as_ref().and_then(|output| std::str::from_utf8(&output.stdout).ok().map(str::trim));
    let cargo_pkg_version = env!("CARGO_PKG_VERSION");

    // Default git_describe to cargo_pkg_version
    let mut git_describe = String::from(cargo_pkg_version);

    if let Some(git_info) = git_info {
        // If the `git_info` contains `CARGO_PKG_VERSION`, we simply use `git_info` as it is.
        // Otherwise, prepend `CARGO_PKG_VERSION` to `git_info`.
        if git_info.contains(cargo_pkg_version) {
            // Remove the 'g' before the commit sha
            let git_info = &git_info.replace('g', "");
            git_describe = git_info.to_string();
        } else {
            git_describe = format!("v{cargo_pkg_version}-{git_info}");
        }
    }

    println!("cargo:rustc-env=_GIT_INFO={git_describe}");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("build_time.rs");

    let now = chrono::Local::now();
    let formatted_time = format!("{}", now.format("%Y-%m-%d %H:%M:%S"));

    let mut f = File::create(dest_path).unwrap();
    write!(f, "pub const BUILD_TIME: &str = \"{formatted_time}\";").unwrap();
}
