use crate::{Error, Sample};
use hound::{ChunksWriter, SampleFormat, WavSpec};
use std::io::{BufWriter, Cursor, Write};

pub fn rebuild(sample: Sample, width: u16) -> Result<Vec<u8>, Error> {
    let data = &sample.data.unwrap()[..(sample.samples * width as usize)];
    let mut writer = BufWriter::new(Cursor::new(Vec::new()));

    let spec = WavSpec {
        channels: sample.channels as u16,
        sample_rate: sample.frequency,
        bits_per_sample: width,
        sample_format: SampleFormat::Int,
    };
    {
        let mut chunk_writer = ChunksWriter::new(&mut writer)?;
        chunk_writer.write_fmt(spec)?;
        {
            let mut embedded_writer = chunk_writer.start_chunk(*b"data")?;
            embedded_writer.write_all(data)?;
        }
        chunk_writer.finalize()?;
    }

    Ok(writer.into_inner().unwrap().into_inner())
}
