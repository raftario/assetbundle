use crate::SoundFormat;
use std::io;
use thiserror::Error;

#[cfg(feature = "pcm")]
use hound;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Expected audio mode in the [0, 16[ range but got `{0}`")]
    SoundFormat(u32),

    #[error("Expected metadata chunk type in the [1, 8[ or [10, 12[ range but got `{0}`")]
    MetadataChunkType(u64),

    #[error("Expected magic header `FBS5` but got `{0:?}`")]
    MagicHeader([u8; 4]),

    #[error("Frequency value `{0}` is not valid and no frequency metadata chunk was provided")]
    Frequency(u32),

    #[error("Non UTF-8 content in name table for sample `{0}`")]
    NameTable(usize),

    #[error("Sample to decode did not originate from the FSB archive decoding it")]
    Mismatched,

    #[error("Decoding samples of type `{0:?}` is not supported")]
    RebuildFormat(SoundFormat),

    #[error("IO error")]
    IO(#[from] io::Error),

    #[cfg(feature = "pcm")]
    #[error("PCM error")]
    PCM(#[from] hound::Error),
}
