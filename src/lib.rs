#![allow(dead_code)]
mod util;

use std::{fs, path::PathBuf, time};

use anyhow::{anyhow, Result};

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

    fn head(&self) -> Result<Hash> {
        let b = fs::read_to_string(self.dir.join("refs").join("heads").join(&self.branch))?;

        Hash::try_from(b.as_str())
    }

    fn read(&self, obj_type: &str, hash: &Hash) -> Result<Vec<u8>> {
        let h = hash.to_string();
        let dir = self.dir.join("objects").join(&h[..2]);
        let obj = dir.join(&h[2..]);

        let bytes = fs::read(obj)?;
        let bytes = util::unzip(bytes)?;

        if !bytes.starts_with(&format!("{obj_type} ").into_bytes()) {
            return Err(anyhow!("not a {obj_type} object"));
        }

        let n = bytes
            .iter()
            .position(|&x| x == 0)
            .ok_or(anyhow!("invalid {obj_type}"))?;
        Ok(bytes[n + 1..].to_vec())
    }

    fn blob(&self, hash: &Hash) -> Result<Vec<u8>> {
        self.read("blob", hash)
    }

    fn tree(&self, hash: &Hash) -> Result<Tree> {
        let mut b = self.read("tree", hash)?;
        let mut blobs = Vec::new();

        loop {
            let parts: Vec<_> = b.splitn(2, |&x| x == 0).collect();
            let fields: Vec<_> = parts[0].splitn(2, |&x| x == b' ').collect();
            blobs.push(Blob {
                name: hex::encode(fields[1]),
                hash: Hash(parts[1][0..20].to_vec()),
            });

            if parts[1].len() == 20 {
                break;
            }

            b = parts[1][20..].to_vec();
        }
        Ok(Tree {
            blobs,
            hash: hash.clone(),
        })
    }

    fn commit(&self, hash: &Hash) -> Result<Commit> {
        let mut commit = Commit {
            msg: "".into(),
            hash: hash.clone(),
            parent: None,
            tree: None,
        };

        let b = self.read("commit", hash)?;
        let lines = b.split(|&x| x == b'\n').collect::<Vec<_>>();
        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                commit.msg = String::from_utf8(lines[i + 1..].concat())? + "\n";
                return Ok(commit);
            }

            let parts = line.splitn(2, |&x| x == b' ').collect::<Vec<_>>();

            match String::from_utf8(parts[0].to_vec())?.as_str() {
                "tree" => {
                    commit.tree = Some(Hash::try_from(
                        String::from_utf8(parts[1].to_vec())?.as_str(),
                    )?);
                }
                "parent" => {
                    commit.parent = Some(Hash::try_from(
                        String::from_utf8(parts[1].to_vec())?.as_str(),
                    )?);
                }
                _ => unreachable!(),
            }
        }

        Ok(commit)
    }
}

struct Commit {
    msg: String,
    hash: Hash,
    parent: Option<Hash>,
    tree: Option<Hash>,
}

struct Tree {
    blobs: Vec<Blob>,
    hash: Hash,
}

struct Blob {
    name: String,
    hash: Hash,
}

#[derive(Clone)]
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
