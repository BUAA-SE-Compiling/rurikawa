use clap::Clap;
mod judge;
mod opt;
mod tester;

fn main() {
    let opt = opt::Opts::parse();
}
