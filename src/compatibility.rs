//! Narrow, opt-out offline fixes. Never rewrite the user's file or match by name.
use ruffle_core::tag_utils::SwfMovieData;
use sha2::{Digest, Sha256};
use std::path::Path;

const ELECTRIC_BOX: &str = "74b098bddd571baffd972d299c3dbd21ff8d809694ff9e307065bcb2d7438736";

pub fn load(path: &Path, enabled: bool) -> Result<SwfMovieData, String> {
    let mut bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if enabled && format!("{:x}", Sha256::digest(&bytes)) == ELECTRIC_BOX {
        bytes = electric_box(&bytes)?;
        eprintln!("[ruffle_libretro] Electric Box: skipped obsolete online sponsor intro");
    }
    let absolute = path.canonicalize().map_err(|e| e.to_string())?;
    let url = url::Url::from_file_path(absolute).map_err(|_| "Invalid SWF path")?;
    SwfMovieData::from_data(&bytes, url.into(), None, None).map_err(|e| e.to_string())
}

fn electric_box(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let buffer = swf::decompress_swf(bytes).map_err(|e| e.to_string())?;
    let tags = &buffer.data;
    let mut cursor = 0;
    while cursor + 2 <= tags.len() {
        let start = cursor;
        let tag = u16::from_le_bytes(tags[cursor..cursor + 2].try_into().unwrap());
        cursor += 2;
        let mut length = (tag & 63) as usize;
        if length == 63 {
            let length_bytes = tags.get(cursor..cursor + 4).ok_or("Truncated SWF tag")?;
            length = u32::from_le_bytes(length_bytes.try_into().unwrap()) as usize;
            cursor += 4;
        }
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= tags.len())
            .ok_or("Invalid SWF tag length")?;
        if tag >> 6 == 12 {
            let action = &tags[cursor..end];
            // Keep game initialization and its original loadGame function;
            // replace only Stop + construction of the dead Candystand stinger.
            if action.len() != 556 || action[490] != 7 || action[555] != 0 {
                return Err("Electric Box profile did not match expected startup script".into());
            }
            let mut replacement = action[..490].to_vec();
            push(&mut replacement, b"\0siteLocked\0\x05\0");
            replacement.push(0x1d); // SetVariable: siteLocked = false
            push(&mut replacement, b"\x07\0\0\0\0\0loadGame\0");
            replacement.extend_from_slice(&[0x3d, 0x17, 0]); // CallFunction, Pop, End
            let mut raw = tags[..start].to_vec();
            raw.extend_from_slice(&((12u16 << 6) | 63).to_le_bytes());
            raw.extend_from_slice(&(replacement.len() as u32).to_le_bytes());
            raw.extend(replacement);
            raw.extend_from_slice(&tags[end..]);
            let mut header = buffer.header.swf_header().clone();
            header.compression = swf::Compression::None;
            let mut output = Vec::new();
            swf::write::write_swf_raw_tags(&header, &raw, &mut output)
                .map_err(|e| e.to_string())?;
            return Ok(output);
        }
        cursor = end;
    }
    Err("Electric Box profile: missing startup script".into())
}

fn push(output: &mut Vec<u8>, payload: &[u8]) {
    output.push(0x96);
    output.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    output.extend_from_slice(payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_movie_is_not_a_candidate() {
        let bytes = include_bytes!("../tests/fixtures/smoke.swf");
        assert_ne!(format!("{:x}", Sha256::digest(bytes)), ELECTRIC_BOX);
        assert!(electric_box(bytes).is_err());
    }
}
