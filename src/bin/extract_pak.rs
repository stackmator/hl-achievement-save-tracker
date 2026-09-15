//! Debug CLI for the shared `.pak` reader in `hl_save_tracker::pak`.

use hl_save_tracker::pak::Pak;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  extract_pak <pak> list [path-substring]   list matching indexed paths");
    eprintln!("  extract_pak <pak> <path> <out-file>        extract one file by full path");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pak_path = &args[1];

    let mut pak = Pak::open(pak_path).expect("open pak");
    eprintln!("pak contains {} indexed files", pak.file_count());

    if args.len() >= 3 && args[2] == "list" {
        let needle = args.get(3).cloned().unwrap_or_default();
        for path in pak.list(&needle) {
            println!("{path}");
        }
        return;
    }

    if args.len() != 4 {
        usage();
    }
    let path_arg = &args[2];
    let out_path = &args[3];

    let entry = pak
        .find(path_arg)
        .cloned()
        .unwrap_or_else(|| panic!("no entry matching '{path_arg}'"));
    eprintln!(
        "found {:?}: size {} -> compressed {}",
        path_arg, entry.uncompressed, entry.compressed
    );

    let out = pak.read(&entry).expect("decompress entry");
    std::fs::write(out_path, &out).expect("write output");
    eprintln!("wrote {} bytes to {}", out.len(), out_path);
}