//! The identifier for a chunk, which is a Blake3 hash of
//! the chunk's bytes.

use std::error::Error;
use std::fmt;
use std::str::FromStr;
use fmt::Formatter;

/// Blake3 hash of the chunk's bytes used to uniquely identify the chunk
/// Raw bytes, never derived from child chunks
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkId([u8; 32]);

impl ChunkId {
    /// Calculate and return the blake3 hash chunk id.
    pub fn calculate(chunk_bytes: &[u8]) -> Self {
        Self(*blake3::hash(chunk_bytes).as_bytes())
    }

    /// construct chunk id from already calculated blake3 hash bytes
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// get the raw byte value of the blake3 hash
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ChunkId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkId(")?;
        let result = fmt_write_bytes(&self.0, f);
        write!(f, ")")?;
        result
    }
}

impl fmt::Debug for ChunkId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkId(")?;
        let result = fmt_write_bytes(&self.0[0..4], f);
        write!(f, ")")?;
        result
    }
}

fn fmt_write_bytes(bytes: &[u8], f: &mut Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{:02x}", byte)?;
    }
    Ok(())
}

impl FromStr for ChunkId {
    type Err = ParseChunkIdError;
    fn from_str(s: &str) -> Result<ChunkId, Self::Err>{
        if !s.is_ascii() {
            return Err(ParseChunkIdError::NotAscii);
        }
        
        if s.len() != 64 {
            return Err(ParseChunkIdError::InvalidLength(s.len()))
        }
        
        let mut bytes: [u8; 32] = [0; 32];
        let raw = s.as_bytes();

        for i in 0..32 {
            let high_char = raw[i * 2] as char;
            let low_char = raw[(i * 2) + 1] as char;
            let high = high_char.to_digit(16)
                .ok_or(ParseChunkIdError::InvalidCharacter(high_char))?;
            let low = low_char.to_digit(16)
                .ok_or(ParseChunkIdError::InvalidCharacter(low_char))?;

            bytes[i] = ((high << 4) | low) as u8;
        }
        
        Ok(ChunkId(bytes))
    }
}

/// Possible errors when parsing a chunk id from a string
#[derive(Debug, Eq, PartialEq)]
pub enum ParseChunkIdError {
    /// str must be of length 64
    InvalidLength(usize),

    /// must be all valid hexadecimal digits
    InvalidCharacter(char),

    /// the input string must be ascii characters
    NotAscii,
}

const PARSE_ERR_INVALID_LENGTH: &str = "Attempted to parse chunkid with invalid length";
const PARSE_ERR_INVALID_CHAR: &str = "Attempted to parse chunkid with invalid character";
const PARSE_ERR_NOT_ASCII: &str = "Chunk Id string must be ascii to parse.";

impl fmt::Display for ParseChunkIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(l) => write!(f, "{PARSE_ERR_INVALID_LENGTH} {l}"),
            Self::InvalidCharacter(c) => write!(f, "{PARSE_ERR_INVALID_CHAR} {c:?}"),
            Self::NotAscii => write!(f, "{PARSE_ERR_NOT_ASCII}"),
        }
    }
}

impl Error for ParseChunkIdError {}

#[cfg(test)]
mod tests {
    use crate::chunk_id::ChunkId;
    use crate::chunk_id::ParseChunkIdError;
    use std::str::FromStr;
    
    const TEST_INPUT_BYTES: [u8; 6] = [0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33];
    const EXPECTED_HASH_STR: &str = "45e60cc2caa1eab44689b8f9a7c7f56cf91a30c8e49b5cb7e76d0dce8134dffe";
    const EXPECTED_DEBUG_HASH_STR: &str = "45e60cc2";
    const EXPECTED_HASH_BYTES: [u8; 32] = [0x45, 0xE6, 0x0C, 0xC2, 0xCA, 0xA1, 0xEA, 0xB4,
  0x46, 0x89, 0xB8, 0xF9, 0xA7, 0xC7, 0xF5, 0x6C,
  0xF9, 0x1A, 0x30, 0xC8, 0xE4, 0x9B, 0x5C, 0xB7,
  0xE7, 0x6D, 0x0D, 0xCE, 0x81, 0x34, 0xDF, 0xFE];

    #[test]
    fn test_calculate() {
        let output: ChunkId = ChunkId::calculate(&TEST_INPUT_BYTES);

        assert_bytes_and_str_representations(&output);
    }

    #[test]
    fn test_from_bytes() {
        let output: ChunkId = ChunkId::from_bytes(EXPECTED_HASH_BYTES);

        assert_bytes_and_str_representations(&output);
    }

    fn assert_bytes_and_str_representations(output: &ChunkId) {
        assert_eq!(*(output.as_bytes()), EXPECTED_HASH_BYTES);

        // Test Debug string output
        assert_eq!(format!("{output:?}"), format!("ChunkId({EXPECTED_DEBUG_HASH_STR})"));
        // Test Display string output
        assert_eq!(output.to_string(), format!("ChunkId({EXPECTED_HASH_STR})"));
    }

    #[test]
    fn test_from_string_happy() {
        let input: &str = "1A2b3C4D5E6f7788991011121314151617181920212223242526272829303132";
        let output: ChunkId = ChunkId::from_str(input)
            .expect("Failed to parse string input into a ChunkId");

        assert_eq!(output.to_string(), format!("ChunkId({})", input.to_lowercase()));
    }

    #[test]
    fn test_from_string_errors() {
        // too long
        let mut input: &str = "1A2b3C4D5E6f77889910111213141516171819202122232425262728293031321";
        assert_eq!(ChunkId::from_str(input), Err(ParseChunkIdError::InvalidLength(65)));

        // too short
        assert_eq!(ChunkId::from_str(&input[0..63]), Err(ParseChunkIdError::InvalidLength(63)));

        // empty
        assert_eq!(ChunkId::from_str(""), Err(ParseChunkIdError::InvalidLength(0)));

        // Invalid Ascii character
        input = "1A2b3C4D5E6f77889910111213141516171819L2122232425262728293031321";
        assert_eq!(ChunkId::from_str(input), Err(ParseChunkIdError::InvalidCharacter('L')));

        // Non Ascii character
        input = "1A2b3C4D5E6f77889910111213141516171819す2122232425262728293031321";
        assert_eq!(ChunkId::from_str(input), Err(ParseChunkIdError::NotAscii));
    }
}



