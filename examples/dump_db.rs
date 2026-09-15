use anyhow::Context;
use hl_save_tracker::{extract_raw_database, fix_sqlite_header};
use std::fs;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).context("usage: dump_db <save.sav> <out.db>")?;
    let out = std::env::args().nth(2).context("usage: dump_db <save.sav> <out.db>")?;
    let data = fs::read(&path)?;
    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;
    fs::write(&out, &db_data)?;
    println!("wrote {}", out);
    Ok(())
}