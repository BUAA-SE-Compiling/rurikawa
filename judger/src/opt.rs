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
    /// The coordinator's uri (include port if needed)
    #[clap(required = true)]
    pub host: String,

    /// Path of temp folder, defaults to ~/.rurikawa/
    #[clap(long = "temp-folder", name = "path")]
    pub temp_folder_path: Option<PathBuf>,

    /// Access token
    #[clap(long, short)]
    pub token: Option<String>,
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
