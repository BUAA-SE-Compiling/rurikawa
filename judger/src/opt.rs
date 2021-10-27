use clap::Parser;
use clap::{crate_authors, crate_license, crate_version};
use std::path::PathBuf;

/// The judger client of the online judging platform Rurikawa OJ.
#[derive(Parser, Debug, Clone)]
#[clap(
    version = crate_version!(),
    author = crate_authors!(),
    license = crate_license!(),
    after_help = "Visit https://github.com/BUAA-SE-Compiling/rurikawa for source.",
)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCmd,

    #[clap(flatten)]
    pub opt: GlobalOpts,
}

#[derive(Parser, Debug, Clone)]
pub struct GlobalOpts {
    #[clap(long, short = 'l')]
    pub log_level: Option<tracing::level_filters::LevelFilter>,
    // #[clap(long = "docker")]
    // pub docker_path: String,
}

#[derive(Parser, Debug, Clone)]
pub enum SubCmd {
    /// Run as a long-running runner instance (which is the only available way to run)
    #[clap(name = "connect")]
    Connect(ConnectSubCmd),

    /// Run a single test job in local environment
    #[clap(name = "run")]
    Run(RunSubCmd),
}

#[derive(Parser, Debug, Clone)]
pub struct ConnectSubCmd {
    /// The coordinator's address (include port if needed).
    /// The previous host will be used if not supplied.
    #[clap(env = "RURIKAWA_HOST")]
    pub host: Option<String>,

    /// Supply or override TLS settings
    #[clap(long, alias = "ssl", env = "RURIKAWA_SSL", env = "RURIKAWA_TLS")]
    pub tls: Option<bool>,

    /// Max task count that can be runned concurrently.
    #[clap(long, short, env = "RURIKAWA_CONCURRENT_TASKS")]
    pub concurrent_tasks: Option<usize>,

    /// Path of temp folder, defaults to ~/.rurikawa/
    #[clap(
        long = "temp-folder",
        long = "path",
        name = "path",
        env = "RURIKAWA_TEMP_FOLDER_PATH"
    )]
    pub temp_folder_path: Option<PathBuf>,

    /// Supply or override existing access token
    #[clap(long, env = "RURIKAWA_ACCESS_TOKEN")]
    pub access_token: Option<String>,

    /// Supply or override existing register token
    #[clap(long, short, env = "RURIKAWA_REGISTER_TOKEN")]
    pub register_token: Option<String>,

    /// Supply or override existing alternate name
    #[clap(long, env = "RURIKAWA_ALTERNATE_NAME")]
    pub name: Option<String>,

    /// Supply or override tags
    #[clap(long, short, env = "RURIKAWA_TAG", use_delimiter = true)]
    pub tag: Option<Vec<String>>,

    /// Force refresh access token if possible. Supply this option to register
    /// this judger as a new judger, and discard all previous data.
    #[clap(long, env = "RURIKAWA_FORCE_REFRESH")]
    pub refresh: bool,

    /// Do not save updated data into config file.
    #[clap(long, env = "RURIKAWA_NO_SAVE")]
    pub no_save: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct RunSubCmd {
    /// The job to run. Either specify a folder where `judge.toml` can be found
    /// in it or its subfolders, or specify a file to be used as `judge.toml`.
    /// Defaults to current folder.
    #[clap(name = "job-path")]
    pub job: Option<PathBuf>,

    /// Configuration file of tests.
    #[clap(long, short, name = "config-file-path")]
    pub config: Option<PathBuf>,
}
