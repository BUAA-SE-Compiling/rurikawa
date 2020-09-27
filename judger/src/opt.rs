use clap::Clap;
use std::path::PathBuf;

#[derive(Clap, Debug, Clone)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCmd,

    #[clap(flatten)]
    pub opt: GlobalOpts,
}

#[derive(Clap, Debug, Clone)]
pub struct GlobalOpts {}

#[derive(Clap, Debug, Clone)]
pub enum SubCmd {
    /// Run as a long-running runner instance
    #[clap(name = "connect")]
    Connect(ConnectSubCmd),

    /// Run a single test job in local environment
    #[clap(name = "run")]
    Run(RunSubCmd),
}

#[derive(Clap, Debug, Clone)]
pub struct ConnectSubCmd {
    /// The coordinator's address (include port if needed).
    /// The previous host will be used if not supplied.
    #[clap()]
    pub host: Option<String>,

    /// Supply or override SSL settings
    #[clap(long, short)]
    pub ssl: Option<bool>,

    /// Max task count that can be runned concurrently.
    #[clap(long, short)]
    pub concurrent_tasks: Option<usize>,

    /// Path of temp folder, defaults to ~/.rurikawa/
    #[clap(long = "temp-folder", name = "path")]
    pub temp_folder_path: Option<PathBuf>,

    /// Supply or override existing access token
    #[clap(long)]
    pub access_token: Option<String>,

    /// Supply or override existing register token
    #[clap(long, short)]
    pub register_token: Option<String>,

    /// Supply or override existing alternate name
    #[clap(long)]
    pub name: Option<String>,

    /// Supply or override tags
    #[clap(long, short)]
    pub tag: Option<Vec<String>>,

    /// Force refresh access token if possible. Supply this option to register
    /// this judger as a new judger, and discard all previous data.
    #[clap(long)]
    pub refresh: bool,

    /// Do not save updated data into config file.
    #[clap(long)]
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
