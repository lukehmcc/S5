use blake3::{Hash, Hasher};

use flutter_rust_bridge::frb;
use flutter_rust_bridge::support::from_vec_to_array;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::Arc;

fn blake3_digest<R: Read>(mut reader: R) -> Result<Hash, anyhow::Error> {
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0; 1048576];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize())
}

pub fn hash_blake3_file(path: String) -> Result<Vec<u8>, anyhow::Error> {
    let input = File::open(path)?;
    let reader = BufReader::new(input);
    let digest = blake3_digest(reader)?;

    Ok(digest.as_bytes().to_vec())
}

pub fn hash_blake3(input: Vec<u8>) -> Result<Vec<u8>, anyhow::Error> {
    let digest = blake3::hash(&input);
    Ok(digest.as_bytes().to_vec())
}

pub fn verify_integrity(
    chunk_bytes: Vec<u8>,
    offset: u64,
    bao_outboard_bytes: Vec<u8>,
    blake3_hash: Vec<u8>,
) -> Result<u8, anyhow::Error> {
    let mut slice_stream = bao::encode::SliceExtractor::new_outboard(
        FakeSeeker::new(&chunk_bytes[..]),
        Cursor::new(&bao_outboard_bytes),
        offset,
        262144,
    );

    let mut decode_stream = bao::decode::SliceDecoder::new(
        &mut slice_stream,
        &bao::Hash::from(from_vec_to_array(blake3_hash)),
        offset,
        262144,
    );
    let mut decoded = Vec::new();
    decode_stream.read_to_end(&mut decoded)?;

    Ok(1)
}

struct FakeSeeker<R: Read> {
    reader: R,
    bytes_read: u64,
}

impl<R: Read> FakeSeeker<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            bytes_read: 0,
        }
    }
}

impl<R: Read> Read for FakeSeeker<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buf)?;
        self.bytes_read += n as u64;
        Ok(n)
    }
}

impl<R: Read> Seek for FakeSeeker<R> {
    fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
        // Do nothing and return the current position.
        Ok(self.bytes_read)
    }
}

pub fn hash_bao_file(path: String) -> Result<BaoResult, anyhow::Error> {
    let input = File::open(path)?;
    let reader = BufReader::new(input);

    let result = hash_bao_file_internal(reader);

    Ok(result.unwrap())
}

pub fn hash_bao_memory(bytes: Vec<u8>) -> Result<BaoResult, anyhow::Error> {
    let result = hash_bao_file_internal(&bytes[..]);

    Ok(result.unwrap())
}

pub struct BaoResult {
    pub hash: Vec<u8>,
    pub outboard: Vec<u8>,
}

fn hash_bao_file_internal<R: Read>(mut reader: R) -> Result<BaoResult, anyhow::Error> {
    let mut encoded_incrementally = Vec::new();

    let encoded_cursor = std::io::Cursor::new(&mut encoded_incrementally);

    let mut encoder = bao::encode::Encoder::new_outboard(encoded_cursor);

    let mut buffer = [0; 262144];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        encoder.write(&buffer[..count]);
    }

    Ok(BaoResult {
        hash: encoder.finalize()?.as_bytes().to_vec(),
        outboard: encoded_incrementally,
    })
}
