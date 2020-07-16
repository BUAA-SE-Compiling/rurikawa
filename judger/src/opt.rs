use clap::Clap;
use std::path::PathBuf;

#[derive(Clap, Debug)]
pub struct Opts {
    #[clap(subcommand)]
    cmd: SubCmd,
}

#[derive(Clap, Debug)]
pub enum SubCmd {
    Server,
    Run {
        #[clap(long, short)]
        job_config: PathBuf,

        #[clap(long, short)]
        judge_config: PathBuf,
    },
}
