mod util;

use std::{fs, path::PathBuf};

use anyhow::Result;

struct Git {
    // where .git is located
    dir: PathBuf,
    // current branch
    branch: String,
}

impl Git {
    fn write(&self, obj_type: &str, b: &[u8]) -> Result<Hash> {
        let mut bytes = format!("{obj_type} {}\x00", b.len()).into_bytes();
        bytes.extend(b);
        let bz = util::zip(&bytes)?;

        let sum = util::sha1_sum(bytes);
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
