#![allow(dead_code)]
mod util;

use std::{fs, path::PathBuf, time};

use anyhow::Result;

struct Git {
    // where .git is located
    dir: PathBuf,
    // current branch
    branch: String,
    user: String,
    email: String,
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

    fn add_blob(&self, data: &[u8]) -> Result<Hash> {
        self.write("blob", data)
    }

    fn add_tree(&self, filename: &str, filedata: &[u8]) -> Result<Hash> {
        let hash = self.add_blob(filedata)?;
        let mut content = format!("100644 {filename}\x00").into_bytes();
        content.extend(&hash.0);
        self.write("tree", &content)
    }

    fn add_commit(
        &self,
        filename: &str,
        data: &[u8],
        parent_hash: Option<Hash>,
        msg: &str,
    ) -> Result<Hash> {
        let hash = self.add_tree(filename, data)?;

        let parent = if let Some(p) = parent_hash {
            format!("parent {}\n", p.to_string())
        } else {
            String::new()
        };

        let t = time::UNIX_EPOCH.elapsed()?.as_secs();

        let content = format!(
            "tree {}\n{}author {} <{}> {} +0000\ncommitter {} <{}> {} +0000\n{}\n",
            hash.to_string(),
            parent,
            &self.user,
            &self.email,
            t,
            &self.user,
            &self.email,
            t,
            msg
        )
        .into_bytes();

        let b = self.write("commit", &content)?;
        self.set_head(&b)?;

        Ok(b)
    }

    fn set_head(&self, hash: &Hash) -> Result<()> {
        let filepath = self.dir.join("ref").join("heads").join(&self.branch);
        fs::write(filepath, hash.to_string().into_bytes())?;
        Ok(())
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
