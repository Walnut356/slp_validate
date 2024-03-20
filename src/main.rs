use clap::Parser;
use slp_validate::*;

#[derive(Parser, Debug)]
#[command(version, about = "Run with a path to a .slp file or directory containing .slp files to check for any errors")]
struct Args {
    path: String,
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Debug).format_timestamp(None).init();
    let args = Args::parse();

    parse(&args.path);
}
