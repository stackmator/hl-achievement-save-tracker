use std::fs::File;
use std::io::Read;

fn main() {
    let path = r"C:\Program Files (x86)\Steam\steamapps\common\Hogwarts Legacy\HogwartsLegacy.exe";

    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    println!("File size: {} bytes", buffer.len());

    // Also search as UTF-16 (wide strings)
    let search_strings_utf16: Vec<Vec<u8>> = [
        "AncientMagic",
        "Ancient_Magic",
        "FGC_Ancient",
        "AMagic_",
        "AncientMagic_001",
        "AncientMagic_000",
        "one_of_each",
        "OneOfEach",
        "FGC_Ancient",
        "AncientMagicKill",
        "AncientMagicDefeat",
        "AncientMagicKillTracking",
        "AncientMagicEnemyKill",
        "AncientMagicEnemyDefeat",
        "AMagic_HN_AN_01",
        "AMagic_HS_AJ_02",
        "AMagic_CO_BA_01",
        "AMagic_CO_AR_01",
        "AMagic_HS_AS_02",
        "AMagic_CO_AS_01",
        "AMagic_HS_BG_01",
    ]
    .iter()
    .map(|s| {
        let mut bytes = Vec::new();
        for c in s.encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        bytes
    })
    .collect();

    println!("=== Searching as UTF-8 ===");
    for &search_str in &[
        "AncientMagic",
        "Ancient_Magic",
        "FGC_Ancient",
        "AMagic_",
        "AncientMagic_001",
        "one_of_each",
        "AMagic_HN_AN_01",
        "AMagic_HS_AJ_02",
        "AMagic_CO_BA_01",
        "AMagic_CO_AR_01",
        "AMagic_HS_AS_02",
        "AMagic_CO_AS_01",
        "AMagic_HS_BG_01",
    ] {
        let bytes = search_str.as_bytes();
        for i in 0..buffer.len().saturating_sub(search_str.len()) {
            if &buffer[i..i + bytes.len()] == bytes {
                println!("Found '{}' at offset 0x{:X} (file offset)", search_str, i);
            }
        }
    }

    println!("\n=== Searching as UTF-16 (wide strings) ===");
    for search_bytes in &search_strings_utf16 {
        for i in 0..buffer.len().saturating_sub(search_bytes.len()) {
            if buffer[i..i + search_bytes.len()] == *search_bytes {
                // Convert bytes back to string for display
                let s = String::from_utf16(
                    &search_bytes.iter().map(|b| *b as u16).collect::<Vec<u16>>(),
                )
                .unwrap_or_default();
                println!("Found UTF-16 '{}' at offset 0x{:X}", s, i);
            }
        }
    }

    // Also search for "AncientMagic" as individual bytes
    println!("\n=== Searching for individual bytes of 'AncientMagic' ===");
    let ancient_magic_bytes = "AncientMagic".as_bytes();
    for i in 0..buffer.len().saturating_sub(ancient_magic_bytes.len()) {
        if &buffer[i..i + ancient_magic_bytes.len()] == ancient_magic_bytes {
            println!("Found 'AncientMagic' at offset 0x{:X}", i);
        }
    }

    // Let's also look at the PE header to understand the structure
    println!("\n=== PE Header Info ===");
    if buffer.len() >= 64 {
        let pe_offset =
            u32::from_le_bytes([buffer[60], buffer[61], buffer[62], buffer[63]]) as usize;
        println!("PE header offset: 0x{:X}", pe_offset);

        if buffer.len() >= pe_offset + 24 {
            let machine = u16::from_le_bytes([buffer[pe_offset + 4], buffer[pe_offset + 5]]);
            let num_sections = u16::from_le_bytes([buffer[pe_offset + 6], buffer[pe_offset + 7]]);
            println!("Machine: 0x{:04X}, Sections: {}", machine, num_sections);
        }
    }
}
