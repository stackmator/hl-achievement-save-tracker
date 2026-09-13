// Sanitizes a save for public distribution: replaces given ASCII strings
// in-place with same-length 'X' masks so the GVAS structure stays intact.
// Re-run whenever regenerating the testdata fixture.
use std::io::Write;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut src = None;
    let mut dst = None;
    let mut scrub: Vec<Vec<u8>> = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--save" => src = args.next(),
            "--out" => dst = args.next(),
            "--scrub" => {
                if let Some(s) = args.next() {
                    if s.is_ascii() {
                        scrub.push(s.as_bytes().to_vec());
                    } else {
                        eprintln!("ignoring non-ASCII scrub target");
                    }
                }
            }
            _ => anyhow::bail!("unknown arg: {}", a),
        }
    }
    let src = src.ok_or_else(|| anyhow::anyhow!("--save required"))?;
    let dst = dst.ok_or_else(|| anyhow::anyhow!("--out required"))?;

    let mut data = std::fs::read(&src)?;
    for target in &scrub {
        let mut n = 0usize;
        let mut i = 0usize;
        while i + target.len() <= data.len() {
            if &data[i..i + target.len()] == target.as_slice() {
                data[i..i + target.len()].fill(b'X');
                n += 1;
                i += target.len();
            } else {
                i += 1;
            }
        }
        println!(
            "scrubbed {:?} ({:?} occurrences)",
            String::from_utf8_lossy(target),
            n
        );
    }

    let mut f = std::fs::File::create(&dst)?;
    f.write_all(&data)?;
    println!("wrote {}", dst);
    Ok(())
}
