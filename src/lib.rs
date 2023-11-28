mod util;

use std::{fs, path::PathBuf, time};

use anyhow::{anyhow, Context, Result};

#[derive(Clone)]
pub struct Hash(Vec<u8>);

pub struct Commit {
    pub msg: String,
    pub hash: Hash,
    pub parent: Option<Hash>,
    pub tree: Option<Hash>,
}

pub struct Tree {
    pub blobs: Vec<Blob>,
    pub hash: Hash,
}

pub struct Blob {
    pub name: String,
    pub hash: Hash,
}

pub struct Git {
    pub dir: PathBuf,
    pub branch: String,
    pub user: String,
    pub email: String,
}

impl TryFrom<&str> for Hash {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(Self(hex::decode(value.trim()).context("decoding hex")?))
    }
}

impl ToString for Hash {
    fn to_string(&self) -> String {
        hex::encode(&self.0)
    }
}

impl Git {
    pub fn init(&self) -> Result<()> {
        let dirs = &[
            ["objects", "info"],
            ["objects", "pack"],
            ["refs", "heads"],
            ["refs", "tags"],
        ];

        for [d1, d2] in dirs {
            fs::create_dir_all(self.dir.join(d1).join(d2))?;
        }
        fs::write(
            self.dir.join("HEAD"),
            format!("ref: refs/heads/{}", self.branch).into_bytes(),
        )?;
        Ok(())
    }

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

    pub fn add_blob(&self, data: &[u8]) -> Result<Hash> {
        self.write("blob", data)
    }

    pub fn add_tree(&self, filename: &str, filedata: &[u8]) -> Result<Hash> {
        let hash = self.add_blob(filedata)?;
        let mut content = format!("100644 {filename}\x00").into_bytes();
        content.extend(&hash.0);
        self.write("tree", &content)
    }

    pub fn add_commit(
        &self,
        filename: &str,
        data: &[u8],
        parent_hash: Option<Hash>,
        msg: &str,
    ) -> Result<Hash> {
        let hash = self.add_tree(filename, data).context("adding hash")?;

        let parent = if let Some(p) = parent_hash {
            format!("parent {}\n", p.to_string())
        } else {
            String::new()
        };

        let t = time::UNIX_EPOCH.elapsed().context("time")?.as_secs();

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

        let b = self.write("commit", &content).context("writing commit")?;
        self.set_head(&b).context("setting head")?;

        Ok(b)
    }

    pub fn set_head(&self, hash: &Hash) -> Result<()> {
        let dir = self.dir.join("ref").join("heads");
        let filepath = dir.join(&self.branch);
        fs::create_dir_all(dir).context("creating dir")?;
        fs::write(filepath, hash.to_string().into_bytes()).context("writing to head")
    }

    pub fn head(&self) -> Result<Hash> {
        let b = fs::read_to_string(self.dir.join("refs").join("heads").join(&self.branch))
            .context("reading dir/refs/heads/branch")?;

        Hash::try_from(b.as_str()).context("conversion")
    }

    pub fn blob(&self, hash: &Hash) -> Result<Vec<u8>> {
        self.read("blob", hash)
    }

    pub fn tree(&self, hash: &Hash) -> Result<Tree> {
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

    pub fn commit(&self, hash: &Hash) -> Result<Commit> {
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
                _ => continue,
            }
        }

        Ok(commit)
    }

    pub fn log(&self) -> Result<Vec<Commit>> {
        let mut hash = Some(self.head().context("reading head")?);

        let mut commits = Vec::new();
        while let Some(ref h) = hash {
            let ci = self.commit(h)?;
            hash = ci.parent.clone();
            commits.push(ci);
        }

        Ok(commits)
    }
}
