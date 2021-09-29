use clap::Clap;
use clap::{crate_authors, crate_license, crate_version};
use std::path::PathBuf;

/// The judger client of the online judging platform Rurikawa OJ.
#[derive(Clap, Debug, Clone)]
#[clap(
    version = crate_version!(),
    author = crate_authors!(),
    license = crate_license!(),
    after_help = "Visit https://github.com/BUAA-SE-Compiling/rurikawa for source.",
    setting = clap::AppSettings::ColoredHelp
)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCmd,

    #[clap(flatten)]
    pub opt: GlobalOpts,
}

#[derive(Clap, Debug, Clone)]
pub struct GlobalOpts {
    #[clap(long, short = 'l', default_value = "info", env = "LOG_LEVEL")]
    pub log_level: tracing::level_filters::LevelFilter,
    // #[clap(long = "docker")]
    // pub docker_path: String,
}

#[derive(Clap, Debug, Clone)]
pub enum SubCmd {
    /// Run as a long-running runner instance (which is the only available way to run)
    #[clap(name = "connect", setting = clap::AppSettings::ColoredHelp)]
    Connect(ConnectSubCmd),

    /// Run a single test job in local environment
    #[clap(name = "run")]
    Run(RunSubCmd),
}

#[derive(Clap, Debug, Clone)]
pub struct ConnectSubCmd {
    /// The coordinator's address (include port if needed).
    /// The previous host will be used if not supplied.
    #[clap(env = "RURIKAWA_HOST")]
    pub host: Option<String>,

    /// Supply or override SSL settings
    #[clap(long, short, env = "RURIKAWA_SSL")]
    pub ssl: Option<bool>,

    /// Max task count that can be runned concurrently.
    #[clap(long, short, env = "RURIKAWA_CONCURRENT_TASKS")]
    pub concurrent_tasks: Option<usize>,

    /// Path of temp folder, defaults to ~/.rurikawa/
    #[clap(long = "temp-folder", name = "path", env = "RURIKAWA_TEMP_FOLDER_PATH")]
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

#[derive(Clap, Debug, Clone)]
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
