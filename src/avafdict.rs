//! Parser for Avalanche's "AVAFDICT 2.0" localization dictionaries, as found
//! in Hogwarts Legacy's `Phoenix/Content/Localization/WIN64/MAIN-*.bin` and
//! `SUB-*.bin` files.
//!
//! Layout (little-endian):
//!
//! ```text
//! 0x00  "AVAFDICT 2.0   \0"        UTF-16 LE magic (32 bytes)
//! 0x20  pair count                 u64  (number of key/value pairs)
//! 0x28  header size                u64  (0x48 = start of dictionary)
//! 0x30  dictionary size            u64
//! 0x38  end of dictionary          u64  (= start of text blob)
//! 0x40  text blob size             u64
//! ```
//!
//! The dictionary starts at `header size` and holds `pair count * 2` entries
//! of 12 bytes each: a u64 offset into the text blob plus a u32 length.
//! Strings in the blob are glued together with no separators, in the same
//! order as the dictionary. Entries alternate key / value pairs.

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::Cursor;

const HEADER_MAGIC_SIZE: usize = 0x20;
const HEADER_ENTRY_COUNT: usize = 0x20;
const HEADER_HEADER_SIZE: usize = 0x28;
const HEADER_TEXT_START: usize = 0x38;
const HEADER_TEXT_SIZE: usize = 0x40;
const ENTRY_SIZE: usize = 12;

/// A parsed AVAFDICT dictionary: an ordered list of (key, value) pairs plus
/// an index for fast key lookups.
#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    pairs: Vec<(String, String)>,
    index: HashMap<String, usize>,
}

impl Dictionary {
    /// Parse an AVAFDICT 2.0 binary blob.
    pub fn parse(data: &[u8]) -> Result<Dictionary> {
        if data.len() < HEADER_MAGIC_SIZE {
            bail!("file too small to be AVAFDICT");
        }
        let magic: Vec<u16> = data[..HEADER_MAGIC_SIZE]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let magic = String::from_utf16_lossy(&magic);
        if !magic.starts_with("AVAFDICT") {
            bail!(
                "not an AVAFDICT file (magic {:?})",
                magic.trim_end_matches('\0')
            );
        }

        let mut r = Cursor::new(data);
        r.set_position(HEADER_ENTRY_COUNT as u64);
        let pair_count = r.read_u64::<LittleEndian>()?;
        let header_size = r.read_u64::<LittleEndian>()? as usize;
        let _dict_size = r.read_u64::<LittleEndian>()?;
        let text_start = r.read_u64::<LittleEndian>()? as usize;
        let text_size = r.read_u64::<LittleEndian>()? as usize;

        let string_count = pair_count
            .checked_mul(2)
            .context("dictionary entry count overflow")? as usize;
        let text_end = text_start
            .checked_add(text_size)
            .context("text blob overflow")?;
        let text = data
            .get(text_start..text_end)
            .with_context(|| format!("text blob [{text_start}..{text_end}) past end of file"))?;

        let mut raw = Vec::with_capacity(string_count);
        for i in 0..string_count {
            let entry_off = header_size + i * ENTRY_SIZE;
            let entry = data
                .get(entry_off..entry_off + ENTRY_SIZE)
                .with_context(|| format!("dictionary entry {i} past end of file"))?;
            let mut e = Cursor::new(entry);
            let str_offset = e.read_u64::<LittleEndian>()? as usize;
            let str_len = e.read_u32::<LittleEndian>()? as usize;
            let s = text
                .get(str_offset..str_offset + str_len)
                .with_context(|| {
                    format!(
                        "entry {i} string [{str_offset}..{}) out of text blob",
                        str_offset + str_len
                    )
                })?;
            raw.push(String::from_utf8_lossy(s).into_owned());
        }

        let mut pairs = Vec::with_capacity(string_count / 2);
        let mut index = HashMap::with_capacity(string_count / 2);
        for chunk in raw.chunks(2) {
            let key = &chunk[0];
            let value = chunk.get(1).cloned().unwrap_or_default();
            if !index.contains_key(key) {
                index.insert(key.clone(), pairs.len());
            }
            pairs.push((key.clone(), value));
        }

        Ok(Dictionary { pairs, index })
    }

    /// Look up the localised value for a key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.index.get(key).map(|&i| self.pairs[i].1.as_str())
    }

    /// All (key, value) pairs in file order.
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}