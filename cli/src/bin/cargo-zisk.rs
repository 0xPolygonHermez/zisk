use anyhow::Result;
use cargo_zisk::commands::ZiskCmd;
use clap::Parser;

fn main() -> Result<()> {
    ZiskCmd::parse().run()
}
