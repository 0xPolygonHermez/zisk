use anyhow::Result;
use cargo_zisk::commands::ZiskDevCmd;
use clap::Parser;

fn main() -> Result<()> {
    ZiskDevCmd::parse().run()
}
