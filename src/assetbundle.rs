use crate::Error;
use byteorder::{BigEndian, ReadBytesExt};
use std::convert::TryInto;
use std::{
    convert::TryFrom,
    io::{BufRead, Read, Seek, SeekFrom},
};

#[derive(Debug, Clone)]
struct AssetBundle {
    signature: Signature,
    format_version: i32,
    unity_version: String,
    generator_version: String,
}

#[derive(Debug, Copy, Clone)]
enum Signature {
    Raw,
    Web,
    FS,
}

impl TryFrom<Vec<u8>> for Signature {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        match value.as_slice() {
            b"UnityRaw" => Ok(Signature::Raw),
            b"UnityWeb" => Ok(Signature::Web),
            b"UnityFS" => Ok(Signature::FS),
            _ => Err(Error::Signature(value)),
        }
    }
}

impl AssetBundle {
    fn load<R: BufRead + Seek>(mut reader: R) -> Result<Self, Error> {
        let mut magic = [0; 5];
        reader.read_exact(&mut magic)?;
        if magic != *b"Unity" {
            return Err(Error::Magic(magic));
        }
        reader.seek(SeekFrom::Start(0));

        let mut signature = Vec::new();
        reader.read_until(0, &mut signature);
        let signature = signature.try_into()?;
        let format_version = reader.read_i32::<BigEndian>()?;
        let mut unity_version = Vec::new();
        reader.read_until(0, &mut unity_version)?;
        let unity_version = String::from_utf8(unity_version)?;
        let mut generator_version = Vec::new();
        reader.read_until(0, &mut generator_version)?;
        let generator_version = String::from_utf8(generator_version)?;

        match signature {
            Signature::FS => Ok(Self),
            Signature::Raw | Signature::Web => Ok(Self),
        }
    }
}
