use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    io::{BufRead, Read, Seek, SeekFrom},
};

mod error;
pub use error::Error;

#[cfg(feature = "pcm")]
mod pcm;

#[derive(Debug, Copy, Clone)]
pub enum SoundFormat {
    None,
    PCM8,
    PCM16,
    PCM24,
    PCM32,
    PCMFloat,
    GCADPCM,
    IMAADPCM,
    VAG,
    HEVAG,
    XMA,
    MPEG,
    CELT,
    AT9,
    XWMA,
    Vorbis,
}

impl SoundFormat {
    pub fn file_extension(self) -> &'static str {
        match self {
            SoundFormat::MPEG => "mp3",
            SoundFormat::Vorbis => "ogg",
            SoundFormat::PCM8 | SoundFormat::PCM16 | SoundFormat::PCM32 => "wav",
            _ => "bin",
        }
    }
}

impl TryFrom<u32> for SoundFormat {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SoundFormat::None),
            1 => Ok(SoundFormat::PCM8),
            2 => Ok(SoundFormat::PCM16),
            3 => Ok(SoundFormat::PCM24),
            4 => Ok(SoundFormat::PCM32),
            5 => Ok(SoundFormat::PCMFloat),
            6 => Ok(SoundFormat::GCADPCM),
            7 => Ok(SoundFormat::IMAADPCM),
            8 => Ok(SoundFormat::VAG),
            9 => Ok(SoundFormat::HEVAG),
            10 => Ok(SoundFormat::XMA),
            11 => Ok(SoundFormat::MPEG),
            12 => Ok(SoundFormat::CELT),
            13 => Ok(SoundFormat::AT9),
            14 => Ok(SoundFormat::XWMA),
            15 => Ok(SoundFormat::Vorbis),
            _ => Err(Error::SoundFormat(value)),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FSB5Header {
    pub id: [u8; 4],
    pub version: u32,
    pub num_samples: usize,
    pub sample_headers_size: usize,
    pub name_table_size: usize,
    pub data_size: usize,
    pub mode: SoundFormat,

    pub zero: [u8; 8],
    pub hash: [u8; 16],
    pub dummy: [u8; 8],

    pub unknown: u32,

    pub size: usize,
}

impl FSB5Header {
    fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        let mut id: [u8; 4] = [0; 4];
        reader.read_exact(&mut id)?;
        let version = reader.read_u32::<LittleEndian>()?;
        let num_samples = reader.read_u32::<LittleEndian>()? as usize;
        let sample_headers_size = reader.read_u32::<LittleEndian>()? as usize;
        let name_table_size = reader.read_u32::<LittleEndian>()? as usize;
        let data_size = reader.read_u32::<LittleEndian>()? as usize;
        let mode = reader.read_u32::<LittleEndian>()?;
        let mut zero = [0; 8];
        reader.read_exact(&mut zero)?;
        let mut hash = [0; 16];
        reader.read_exact(&mut hash)?;
        let mut dummy = [0; 8];
        reader.read_exact(&mut dummy)?;
        let unknown = match version {
            0 => reader.read_u32::<LittleEndian>()?,
            _ => 0,
        };
        let mode = mode.try_into()?;
        let size = reader.seek(SeekFrom::Current(0))? as usize;

        Ok(Self {
            id,
            version,
            num_samples,
            sample_headers_size,
            name_table_size,
            data_size,
            mode,
            zero,
            hash,
            dummy,
            unknown,
            size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub frequency: u32,
    pub channels: u64,
    pub data_offset: usize,
    pub samples: usize,

    pub metadata: HashMap<u64, MetadataChunk>,

    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum MetadataChunk {
    Channels(u8),
    Frequency(u32),
    Loop(u32, u32),
    XMASeek(Vec<u8>),
    DSPCOEFF(Vec<u8>),
    XWMAData(Vec<u8>),
    VorbisData { crc32: u32, unknown: Vec<u8> },
}

impl MetadataChunk {
    fn read<R: Read>(reader: &mut R, chunk_size: usize, chunk_type: u64) -> Result<Self, Error> {
        match chunk_type {
            1 => {
                let channels = reader.read_u8()?;
                Ok(MetadataChunk::Channels(channels))
            }
            2 => {
                let frequency = reader.read_u32::<LittleEndian>()?;
                Ok(MetadataChunk::Frequency(frequency))
            }
            3 => {
                let loop_tuple = (
                    reader.read_u32::<LittleEndian>()?,
                    reader.read_u32::<LittleEndian>()?,
                );
                Ok(MetadataChunk::Loop(loop_tuple.0, loop_tuple.1))
            }
            6 => {
                let mut data = vec![0; chunk_size];
                reader.read_exact(&mut data)?;
                Ok(MetadataChunk::XMASeek(data.to_vec()))
            }
            7 => {
                let mut data = vec![0; chunk_size];
                reader.read_exact(&mut data)?;
                Ok(MetadataChunk::DSPCOEFF(data.to_vec()))
            }
            10 => {
                let mut data = vec![0; chunk_size];
                reader.read_exact(&mut data)?;
                Ok(MetadataChunk::XWMAData(data.to_vec()))
            }
            11 => {
                let crc32 = reader.read_u32::<LittleEndian>()?;
                let mut unknown = vec![0; chunk_size];
                reader.read_exact(&mut unknown)?;
                Ok(MetadataChunk::VorbisData {
                    crc32,
                    unknown: unknown.to_vec(),
                })
            }
            _ => Err(Error::MetadataChunkType(chunk_type)),
        }
    }
}

fn bits(val: u64, start: u64, len: u64) -> u64 {
    let stop = start + len;
    let r = val & ((1 << stop) - 1);
    r >> start
}

#[derive(Debug, Clone)]
pub struct FSB5 {
    pub header: FSB5Header,
    pub raw_size: usize,
    pub samples: Vec<Sample>,
}

impl FSB5 {
    pub fn read<R: BufRead + Seek>(mut reader: R) -> Result<Self, Error> {
        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;
        if magic != *b"FSB5" {
            return Err(Error::MagicHeader(magic));
        }

        reader.seek(SeekFrom::Start(0))?;
        let header = FSB5Header::read(&mut reader)?;

        let raw_size =
            header.size + header.sample_headers_size + header.name_table_size + header.data_size;

        let mut samples = Vec::with_capacity(header.num_samples);
        for i in 0..header.num_samples {
            let mut raw = reader.read_u64::<LittleEndian>()?;
            let mut next_chunk = bits(raw, 0, 1);
            let mut frequency = bits(raw, 1, 4) as u32;
            let channels = bits(raw, 1 + 4, 1) + 1;
            let data_offset = (bits(raw, 1 + 4 + 1, 28) * 16) as usize;
            let self_samples = bits(raw, 1 + 4 + 1 + 28, 30) as usize;

            let mut chunks = HashMap::new();
            while next_chunk != 0 {
                raw = reader.read_u32::<LittleEndian>()? as u64;
                next_chunk = bits(raw, 0, 1);
                let chunk_size = bits(raw, 1, 24) as usize;
                let chunk_type = bits(raw, 1 + 24, 7);

                let chunk_data = match MetadataChunk::read(&mut reader, chunk_size, chunk_type) {
                    Ok(cd) => cd,
                    Err(e) => match e {
                        Error::MetadataChunkType(_) => {
                            eprintln!("{}", e);
                            continue;
                        }
                        _ => return Err(e),
                    },
                };

                chunks.insert(chunk_type, chunk_data);
            }

            if let Some(MetadataChunk::Frequency(f)) = chunks.get(&2) {
                frequency = *f;
            } else {
                frequency = match frequency {
                    1 => 8000,
                    2 => 11000,
                    3 => 11025,
                    4 => 16000,
                    5 => 22050,
                    6 => 24000,
                    7 => 32000,
                    8 => 44100,
                    9 => 48000,
                    _ => {
                        return Err(Error::Frequency(frequency));
                    }
                }
            }

            samples.push(Sample {
                name: format!("{}", i),
                frequency,
                channels,
                data_offset,
                samples: self_samples,
                metadata: chunks,
                data: None,
            });
        }

        if header.name_table_size > 0 {
            let nametable_start = reader.seek(SeekFrom::Current(0))? as usize;

            let mut samplename_offsets = vec![0; header.num_samples];
            for i in samplename_offsets.iter_mut() {
                *i = reader.read_u32::<LittleEndian>()? as usize;
            }

            for (i, sample) in samples.iter_mut().enumerate() {
                reader.seek(SeekFrom::Start(
                    (nametable_start + samplename_offsets[i]) as u64,
                ))?;
                let mut name = Vec::new();
                reader.read_until(0, &mut name)?;
                sample.name = String::from_utf8(name).map_err(|_| Error::NameTable(i))?;
            }
        }

        reader.seek(SeekFrom::Start(
            (header.size + header.sample_headers_size + header.name_table_size) as u64,
        ))?;
        for i in 0..header.num_samples {
            let data_start = samples.get(i).unwrap().data_offset;
            let data_end = if i < header.num_samples - 1 {
                samples.get(i + 1).unwrap().data_offset
            } else {
                data_start + header.data_size
            };
            let mut data = Vec::with_capacity(data_end - data_start);
            reader.read_exact(&mut data)?;
            samples.get_mut(i).unwrap().data = Some(data);
        }

        Ok(Self {
            header,
            raw_size,
            samples,
        })
    }

    pub fn rebuild(&self, sample: Sample) -> Result<Vec<u8>, Error> {
        match self.header.mode {
            SoundFormat::MPEG => Ok(sample.data.unwrap()),
            #[cfg(feature = "pcm")]
            SoundFormat::PCM8 => pcm::rebuild(sample, 1),
            #[cfg(feature = "pcm")]
            SoundFormat::PCM16 => pcm::rebuild(sample, 2),
            #[cfg(feature = "pcm")]
            SoundFormat::PCM32 => pcm::rebuild(sample, 4),
            _ => Err(Error::RebuildFormat(self.header.mode)),
        }
    }
}
