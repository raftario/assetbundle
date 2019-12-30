use crate::Error;
use std::convert::TryFrom;

enum CompressionType {
    None,
    LZMA,
    LZ4,
    LZ4HC,
    LZ4AM,
}

impl TryFrom<u32> for CompressionType {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::LZMA),
            2 => Ok(CompressionType::LZ4),
            3 => Ok(CompressionType::LZ4HC),
            4 => Ok(CompressionType::LZ4AM),
            _ => Err(Error::CompressionType(value)),
        }
    }
}
