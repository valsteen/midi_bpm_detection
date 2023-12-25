use build::{get_config_dir, get_data_dir};

pub mod dispatch;

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");

#[must_use]
pub fn version() -> String {
    let author = clap::crate_authors!();

    let commit_hash = GIT_COMMIT_HASH;

    // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{commit_hash}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}
