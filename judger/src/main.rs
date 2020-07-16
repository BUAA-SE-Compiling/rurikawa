use clap::Clap;
use tokio::prelude::*;
mod opt;

fn main() {
    let opt = opt::Opts::parse();
    let mut rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async_main());
}

async fn async_main() {}
