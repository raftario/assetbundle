use crate::Error;
use byteorder::{BigEndian, ReadBytesExt};
use std::convert::TryInto;
use std::{
    convert::TryFrom,
    io::{BufRead, Read, Seek, SeekFrom},
};

#[derive(Debug, Clone)]
struct PartialAssetBundleHeader {
    signature: Signature,
    format_version: i32,
    unity_version: String,
    generator_version: String,
}

#[derive(Debug, Clone)]
enum AssetBundleHeader {
    FS {
        signature: Signature,
        format_version: i32,
        unity_version: String,
        generator_version: String,
        file_size: usize,
        ciblock_size: usize,
        uiblock_size: usize,
    },
    Raw {
        signature: Signature,
        format_version: i32,
        unity_version: String,
        generator_version: String,
        file_size: usize,
        header_size: usize,
        file_count: usize,
        bundle_count: usize,
        bundle_size: Option<usize>,
        uncompressed_bundle_size: Option<usize>,
        compressed_file_size: Option<usize>,
        asset_header_size: Option<usize>,
        name: String,
    },
}

#[derive(Debug, Clone)]
struct AssetBundle {
    header: AssetBundleHeader,
    assets: Vec<Asset>,
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
        reader.seek(SeekFrom::Start(0))?;

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
        let header = PartialAssetBundleHeader {
            signature,
            format_version,
            unity_version,
            generator_version,
        };

        match signature {
            Signature::FS => Self::load_raw(reader, header),
            Signature::Raw | Signature::Web => Self::load_unityfs(reader, header),
        }
    }

    fn load_raw<R: BufRead + Seek>(
        mut reader: R,
        header: PartialAssetBundleHeader,
    ) -> Result<Self, Error> {
        let file_size = reader.read_u32::<BigEndian>()? as usize;
        let header_size = reader.read_i32::<BigEndian>()? as usize;

        let file_count = reader.read_i32::<BigEndian>()? as usize;
        let bundle_count = reader.read_i32::<BigEndian>()? as usize;

        let mut bundle_size = None;
        let mut uncompressed_bundle_size = None;
        if header.format_version >= 2 {
            bundle_size = Some(reader.read_u32::<BigEndian>()? as usize);

            if header.format_version >= 3 {
                uncompressed_bundle_size = Some(reader.read_u32::<BigEndian>()? as usize);
            }
        }

        let mut compressed_file_size = None;
        let mut asset_header_size = None;
        if header_size >= 60 {
            compressed_file_size = Some(reader.read_u32::<BigEndian>()? as usize);
            asset_header_size = Some(reader.read_u32::<BigEndian>()? as usize);
        }

        reader.read_i32::<BigEndian>()?;
        reader.read_u8()?;
        let mut name = Vec::new();
        reader.read_until(0, &mut name);
        let name = String::from_utf8(name)?;

        reader.seek(SeekFrom::Start(header_size as u64))?;
        let num_assets = match header.signature {
            Signature::Raw => reader.read_i32::<BigEndian>() as usize,
            Signature::Web => 1,
            _ => unreachable!(),
        };
        let header = AssetBundleHeader::Raw {
            signature: header.signature,
            format_version: header.format_version,
            unity_version: header.unity_version,
            generator_version: header.generator_version,
            file_size,
            header_size,
            file_count,
            bundle_count,
            bundle_size,
            uncompressed_bundle_size,
            compressed_file_size,
            asset_header_size,
            name,
        };
        let mut assets = Vec::with_capacity(num_assets);
        for i in 0..num_assets {
            let asset = Asset::read(&header, &mut reader)?;
            assets.push(asset);
        }

        Ok(Self { header, assets })
    }

    fn load_unityfs<R: Read + Seek>(
        mut reader: R,
        header: AssetBundleHeader,
    ) -> Result<Self, Error> {
        let file_size = reader.read_i64::<BigEndian>()? as usize;
        let ciblock_size = reader.read_u32::<BigEndian>()? as usize;
        let uiblock_size = reader.read_u32::<BigEndian>()? as usize;
        let flags = reader.read_u32::<BigEndian>()?;
        let compression = (flags & 0x3F).try_into()?;
        let eof_metadata = flags & 0x80;
        let mut orig_pos = None;
        if eof_metadata != 0 {
            orig_pos = Some(reader.seek(SeekFrom::Current(0))?) as u64;
            reader.seek(SeekFrom::End(-ciblock_size as i64))?;
        }
        // TODO
        if eof_metadata != 0 {
            reader.seek(SeekFrom::Start(orig_pos.unwrap()))?;
        }

        Ok(Self)
    }
}
