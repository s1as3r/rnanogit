use anyhow::Result;
use flate2::{
    write::{ZlibDecoder, ZlibEncoder},
    Compression,
};
use sha1::{Digest, Sha1};
use std::io::Write;

pub(crate) fn zip<T: AsRef<[u8]>>(content: T) -> Result<Vec<u8>> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(content.as_ref())?;
    Ok(e.finish()?)
}

pub(crate) fn unzip<T: AsRef<[u8]>>(content: T) -> Result<Vec<u8>> {
    let mut d = ZlibDecoder::new(Vec::new());
    d.write_all(content.as_ref())?;

    Ok(d.finish()?)
}

pub(crate) fn sha1_sum<T: AsRef<[u8]>>(content: T) -> Vec<u8> {
    let mut hasher = Sha1::new();

    hasher.update(content);

    hasher.finalize().to_vec()
}
