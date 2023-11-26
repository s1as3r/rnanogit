use std::{fs, path::PathBuf};

use anyhow::Result;

struct Git {
    // where .git is located
    dir: PathBuf,
    // current branch
    branch: String,
}

impl Git {
    fn write(&self, obj_type: &str, mut b: Vec<u8>) -> Result<Hash> {
        b.extend(format!("{obj_type} {}\x00", b.len()).into_bytes());
        let bz = util::zip(&b)?;

        let sum = util::sha1_sum(b);
        let hash = hex::encode(&sum);

        let dir = self.dir.join("objects").join(&hash[..2]);
        let obj = dir.join(&hash[2..]);

        fs::create_dir_all(dir)?;
        fs::write(obj, bz)?;

        Ok(Hash(sum))
    }
}

struct Hash(Vec<u8>);

impl TryFrom<&str> for Hash {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(Self(hex::decode(value)?))
    }
}

impl ToString for Hash {
    fn to_string(&self) -> String {
        hex::encode(&self.0)
    }
}

mod util {
    use anyhow::Result;
    use flate2::{
        write::{ZlibDecoder, ZlibEncoder},
        Compression,
    };
    use sha1::{Digest, Sha1};
    use std::io::Write;

    pub fn zip<T: AsRef<[u8]>>(content: T) -> Result<Vec<u8>> {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(content.as_ref())?;
        Ok(e.finish()?)
    }

    pub fn uzip<T: AsRef<[u8]>>(content: T) -> Result<Vec<u8>> {
        let mut d = ZlibDecoder::new(Vec::new());
        d.write_all(content.as_ref())?;

        Ok(d.finish()?)
    }

    pub fn sha1_sum<T: AsRef<[u8]>>(content: T) -> Vec<u8> {
        let mut hasher = Sha1::new();

        hasher.update(content);

        hasher.finalize().to_vec()
    }
}
