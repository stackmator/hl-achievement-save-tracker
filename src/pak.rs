//! Minimal Unreal Engine 4 .pak reader (pak V11 / Fnv64BugFix, as used by
//! Hogwarts Legacy / UE 4.27).
//!
//! Parses the pak footer + index enough to locate and decompress a single
//! file by path. Compression methods handled: None (stored), Zlib, and Oodle
//! (via the `oozextract` crate).

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use oozextract::Extractor;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

const MAGIC: u32 = 0x5A6F12E1;

const SQLITE_DB_PATH: &str = "Phoenix/Content/SQLiteDB/PhoenixGameData.sqlite";
const EN_LOCRES_PATH: &str = "Phoenix/Content/Localization/WIN64/MAIN-enUS.bin";

/// Pak footer layout is version dependent. We parse V11 (Fnv64BugFix).
fn footer_size() -> usize {
    4 + 4 + 8 + 8 + 20 // magic, version, index_offset, index_size, index hash
        + 16 // encryption uuid
        + 1 // encrypted byte
        + 5 * 32 // compression method names (5 slots for V8B+)
}

/// Convenience paths the tooling cares about.
pub fn sqlite_db_path() -> &'static str {
    SQLITE_DB_PATH
}

pub fn en_locres_path() -> &'static str {
    EN_LOCRES_PATH
}

#[derive(Debug)]
struct Footer {
    encrypted: bool,
    index_offset: u64,
    index_size: u64,
    compression: Vec<Option<Compression>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compression {
    Zlib,
    Oodle,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub offset: u64,
    pub compressed: u64,
    pub uncompressed: u64,
    compression_slot: Option<u32>,
    blocks: Option<Vec<(u64, u64)>>,
    flags: u8,
    compression_block_size: u32,
}

impl Entry {
    fn is_encrypted(&self) -> bool {
        0 != (self.flags & 1)
    }
    fn is_deleted(&self) -> bool {
        0 != (self.flags >> 1) & 1
    }

    /// Full-width entry serialized size (header stored both in the index and
    /// inline in front of the data on disk).
    fn serialized_size(compression: Option<u32>, block_count: u32) -> u64 {
        let mut size = 0;
        size += 8; // offset
        size += 8; // compressed
        size += 8; // uncompressed
        size += 4; // compression index (u32 for V8B+)
        size += 20; // hash
        size += match compression {
            Some(_) => 4 + 16 * block_count as u64, // blocks
            None => 0,
        };
        size += 1; // flags/encrypted
        size += 4; // compression block size
        size
    }

    fn read(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let offset = reader.read_u64::<LittleEndian>()?;
        let compressed = reader.read_u64::<LittleEndian>()?;
        let uncompressed = reader.read_u64::<LittleEndian>()?;
        let compression = match reader.read_u32::<LittleEndian>()? {
            0 => None,
            n => Some(n - 1),
        };
        let mut hash = [0u8; 20];
        reader.read_exact(&mut hash)?;
        let blocks = match compression {
            Some(_) => {
                let n = reader.read_u32::<LittleEndian>()?;
                let mut blocks = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    let start = reader.read_u64::<LittleEndian>()?;
                    let end = reader.read_u64::<LittleEndian>()?;
                    blocks.push((start, end));
                }
                Some(blocks)
            }
            None => None,
        };
        let flags = reader.read_u8()?;
        let compression_block_size = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            offset,
            compressed,
            uncompressed,
            compression_slot: compression,
            blocks,
            flags,
            compression_block_size,
        })
    }

    /// V10+ "encoded" index entry (packed into u32 bitfield + varints).
    fn read_encoded(reader: &mut (impl Read + Seek)) -> Result<Self> {
        let bits = reader.read_u32::<LittleEndian>()?;
        let compression = match (bits >> 23) & 0x3f {
            0 => None,
            n => Some(n - 1),
        };
        let encrypted = (bits & (1 << 22)) != 0;
        let block_count: u32 = (bits >> 6) & 0xffff;
        let mut block_size = bits & 0x3f;
        if block_size == 0x3f {
            block_size = reader.read_u32::<LittleEndian>()?;
        } else {
            block_size <<= 11;
        }

        let mut var_int = |bit: u32| -> Result<u64> {
            Ok(if (bits & (1 << bit)) != 0 {
                reader.read_u32::<LittleEndian>()? as u64
            } else {
                reader.read_u64::<LittleEndian>()?
            })
        };

        let offset = var_int(31)?;
        let uncompressed = var_int(30)?;
        let compressed = match compression {
            None => uncompressed,
            _ => var_int(29)?,
        };

        let offset_base = Self::serialized_size(compression, block_count);

        let blocks = if block_count == 1 && !encrypted {
            Some(vec![(offset_base, offset_base + compressed)])
        } else if block_count > 0 {
            let mut index = offset_base;
            let mut blocks = Vec::with_capacity(block_count as usize);
            for _ in 0..block_count {
                let mut block_size = reader.read_u32::<LittleEndian>()? as u64;
                let block = (index, index + block_size);
                if encrypted {
                    block_size = (block_size + 15) & !15;
                }
                index += block_size;
                blocks.push(block);
            }
            Some(blocks)
        } else {
            None
        };

        Ok(Self {
            offset,
            compressed,
            uncompressed,
            compression_slot: compression,
            blocks,
            flags: encrypted as u8,
            compression_block_size: block_size,
        })
    }
}

fn read_string(reader: &mut (impl Read + Seek)) -> Result<String> {
    let len = reader.read_i32::<LittleEndian>()?;
    if len < 0 {
        let chars = (-len) as usize;
        let mut bytes = vec![0u8; chars * 2];
        reader.read_exact(&mut bytes)?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
        Ok(String::from_utf16_lossy(&units[..end]))
    } else {
        let mut bytes = vec![0u8; len as usize];
        reader.read_exact(&mut bytes)?;
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}

fn read_footer(file: &mut File) -> Result<Footer> {
    let size = footer_size();
    let file_len = file.metadata()?.len();
    file.seek(SeekFrom::Start(file_len - size as u64))?;

    let mut uuid = [0u8; 16];
    file.read_exact(&mut uuid)?;
    let encrypted = read_u8_bool(file)?;
    let magic = file.read_u32::<LittleEndian>()?;
    let version = file.read_u32::<LittleEndian>()?;
    let index_offset = file.read_u64::<LittleEndian>()?;
    let index_size = file.read_u64::<LittleEndian>()?;
    let mut hash = [0u8; 20];
    file.read_exact(&mut hash)?;

    if magic != MAGIC {
        bail!("bad magic 0x{magic:08x} at footer");
    }
    if version != 11 {
        bail!("unsupported pak version {version} (only V11/Fnv64BugFix supported)");
    }

    let mut compression = Vec::with_capacity(5);
    for _ in 0..5 {
        let mut name = [0u8; 32];
        file.read_exact(&mut name)?;
        let name = std::str::from_utf8(&name)?.trim_end_matches('\0');
        let comp = match name {
            "" => None,
            "Zlib" => Some(Compression::Zlib),
            "Oodle" => Some(Compression::Oodle),
            other => bail!("unhandled compression method '{other}'"),
        };
        compression.push(comp);
    }

    Ok(Footer {
        encrypted,
        index_offset,
        index_size,
        compression,
    })
}

fn read_u8_bool(reader: &mut (impl Read + Seek)) -> Result<bool> {
    Ok(reader.read_u8()? == 1)
}

fn parse_index(file: &mut File, footer: &Footer) -> Result<Vec<(String, Entry)>> {
    file.seek(SeekFrom::Start(footer.index_offset))?;
    let mut index_bytes = vec![0u8; footer.index_size as usize];
    file.read_exact(&mut index_bytes)?;
    let mut index = Cursor::new(index_bytes.as_slice());

    let _mount_point = read_string(&mut index).context("mount point")?;
    let _record_count = index.read_u32::<LittleEndian>()?;
    let _path_hash_seed = index.read_u64::<LittleEndian>()?;

    // V10/V11 PathHashIndex layout.
    let has_phi = index.read_u32::<LittleEndian>()?;
    let mut phi = None;
    if has_phi != 0 {
        let phi_offset = index.read_u64::<LittleEndian>()?;
        let phi_size = index.read_u64::<LittleEndian>()?;
        let mut phi_hash = [0u8; 20];
        index.read_exact(&mut phi_hash)?;
        file.seek(SeekFrom::Start(phi_offset))?;
        let mut phi_bytes = vec![0u8; phi_size as usize];
        file.read_exact(&mut phi_bytes)?;
        let mut phi_reader = Cursor::new(phi_bytes);
        let n = phi_reader.read_u32::<LittleEndian>()?;
        let mut entries = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let hash = phi_reader.read_u64::<LittleEndian>()?;
            let encoded_offset = phi_reader.read_i32::<LittleEndian>()?;
            entries.push((hash, encoded_offset));
        }
        phi = Some(entries);
    }

    let has_fdi = index.read_u32::<LittleEndian>()?;
    let mut fdi = None;
    if has_fdi != 0 {
        let fdi_offset = index.read_u64::<LittleEndian>()?;
        let fdi_size = index.read_u64::<LittleEndian>()?;
        let mut fdi_hash = [0u8; 20];
        index.read_exact(&mut fdi_hash)?;
        file.seek(SeekFrom::Start(fdi_offset))?;
        let mut fdi_bytes = vec![0u8; fdi_size as usize];
        file.read_exact(&mut fdi_bytes)?;
        let mut fdi_reader = Cursor::new(fdi_bytes);
        let dir_count = fdi_reader.read_u32::<LittleEndian>()?;
        let mut directories = Vec::with_capacity(dir_count as usize);
        for _ in 0..dir_count {
            let dir_name = read_string(&mut fdi_reader)?;
            let file_count = fdi_reader.read_u32::<LittleEndian>()?;
            let mut files = Vec::with_capacity(file_count as usize);
            for _ in 0..file_count {
                let file_name = read_string(&mut fdi_reader)?;
                let encoded_offset = fdi_reader.read_i32::<LittleEndian>()?;
                files.push((file_name, encoded_offset));
            }
            directories.push((dir_name, files));
        }
        fdi = Some(directories);
    }

    let encoded_size = index.read_u32::<LittleEndian>()?;
    let mut encoded_bytes = vec![0u8; encoded_size as usize];
    index.read_exact(&mut encoded_bytes)?;

    let non_encoded_count = index.read_u32::<LittleEndian>()?;
    let mut non_encoded = Vec::with_capacity(non_encoded_count as usize);
    for _ in 0..non_encoded_count {
        non_encoded.push(Entry::read(&mut index)?);
    }

    let mut entries = Vec::new();
    match fdi {
        Some(directories) => {
            for (dir_name, files) in directories {
                for (file_name, encoded_offset) in files {
                    if encoded_offset == i32::MIN {
                        continue; // deleted/pruned sentinel
                    }
                    let entry = if encoded_offset >= 0 {
                        let mut enc = Cursor::new(encoded_bytes.as_slice());
                        enc.set_position(encoded_offset as u64);
                        Entry::read_encoded(&mut enc)?
                    } else {
                        let idx = (-encoded_offset) as usize - 1;
                        non_encoded[idx].clone()
                    };
                    let path = format!("{}{}", dir_name.trim_start_matches('/'), file_name);
                    entries.push((path, entry));
                }
            }
        }
        None => {
            for entry in non_encoded.into_iter() {
                entries.push((String::new(), entry));
            }
        }
    }
    let _ = phi;
    Ok(entries)
}

fn decompress_entry(file: &mut File, footer: &Footer, entry: &Entry) -> Result<Vec<u8>> {
    if entry.is_encrypted() {
        bail!(
            "entry at offset {} is AES-encrypted; an encryption key is required",
            entry.offset
        );
    }
    if entry.is_deleted() {
        bail!("entry at offset {} is marked deleted", entry.offset);
    }

    // The on-disk record stores a full entry header right before its data.
    // Read it (as repak does) so the data region starts exactly where the
    // index promised.
    file.seek(SeekFrom::Start(entry.offset))?;
    let _stored = Entry::read(file)?;
    let data_offset = file.stream_position()?;
    let header_size = data_offset - entry.offset;

    let mut data = vec![0u8; entry.compressed as usize];
    file.read_exact(&mut data)?;

    let ranges: Vec<std::ops::Range<usize>> = match &entry.blocks {
        Some(blocks) => blocks
            .iter()
            .map(|&(start, end)| {
                let rel = |x: u64| (x - header_size) as usize;
                rel(start)..rel(end)
            })
            .collect(),
        None => vec![0..data.len()],
    };

    match entry
        .compression_slot
        .and_then(|c| footer.compression.get(c as usize).copied().flatten())
    {
        None => {
            if entry.blocks.is_none() {
                if data.len() != entry.uncompressed as usize {
                    bail!(
                        "stored size {} != uncompressed {}",
                        data.len(),
                        entry.uncompressed
                    );
                }
                return Ok(data);
            }
            bail!("uncompressed entry unexpectedly has blocks");
        }
        Some(Compression::Zlib) => {
            let mut out = Vec::with_capacity(entry.uncompressed as usize);
            for range in ranges {
                use flate2::read::ZlibDecoder;
                let mut dec = ZlibDecoder::new(&data[range]);
                std::io::copy(&mut dec, &mut out)?;
            }
            out.resize(entry.uncompressed as usize, 0);
            Ok(out)
        }
        Some(Compression::Oodle) => {
            let chunk_size = if ranges.len() == 1 {
                entry.uncompressed as usize
            } else {
                entry.compression_block_size as usize
            };
            let mut extractor = Extractor::new();
            let mut out = vec![0u8; entry.uncompressed as usize];
            for (decomp_chunk, range) in out.chunks_mut(chunk_size).zip(ranges.iter()) {
                let n = extractor
                    .read_from_slice(&data[range.clone()], decomp_chunk)
                    .with_context(|| {
                        format!(
                            "Oodle decompression failed on block ({}..{})",
                            range.start, range.end
                        )
                    })?;
                if n != decomp_chunk.len() {
                    bail!("block produced {n} bytes, expected {}", decomp_chunk.len());
                }
            }
            Ok(out)
        }
    }
}

/// An opened pak with its file index loaded.
pub struct Pak {
    file: File,
    footer: Footer,
    entries: Vec<(String, Entry)>,
}

impl Pak {
    /// Open a pak file and parse its index.
    pub fn open(path: &str) -> Result<Pak> {
        let mut file = File::open(path).with_context(|| format!("open pak {path}"))?;
        let footer = read_footer(&mut file)?;
        let entries = parse_index(&mut file, &footer)?;
        Ok(Pak {
            file,
            footer,
            entries,
        })
    }

    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    /// Find an entry whose full path equals `path` (case-insensitive) or,
    /// failing that, whose path contains `path` as a substring.
    pub fn find(&self, path: &str) -> Option<&Entry> {
        let needle = path.trim_start_matches('/').to_lowercase();
        self.entries
            .iter()
            .find(|(p, _)| p.to_lowercase() == needle)
            .map(|(_, e)| e)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|(p, _)| p.to_lowercase().contains(&needle))
                    .map(|(_, e)| e)
            })
    }

    /// List all paths containing `substring` (case-insensitive). Empty matches all.
    pub fn list(&self, substring: &str) -> Vec<&str> {
        let needle = substring.to_lowercase();
        self.entries
            .iter()
            .filter(|(p, _)| needle.is_empty() || p.to_lowercase().contains(&needle))
            .map(|(p, _)| p.as_str())
            .collect()
    }

    /// Decompress a single entry into memory.
    pub fn read(&mut self, entry: &Entry) -> Result<Vec<u8>> {
        decompress_entry(&mut self.file, &self.footer, entry)
    }
}