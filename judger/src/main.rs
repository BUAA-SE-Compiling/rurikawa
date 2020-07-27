use clap::Clap;
use tokio::prelude::*;
mod opt;

#[tokio::main]
async fn main() {
    let opt = opt::Opts::parse();
    println!("Hello world");
}
