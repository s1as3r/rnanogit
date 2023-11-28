use std::{
    env,
    io::{self, Read},
};

use anyhow::{anyhow, Context, Result};
use rnanogit::Git;

fn main() -> Result<()> {
    let git = Git {
        dir: ".git".into(),
        branch: "master".into(),
        user: "rnanogit".into(),
        email: "someemail@rnanogitexample.com".into(),
    };

    let mut args = env::args();
    if args.len() < 2 {
        return Err(anyhow!("USAGE: rnanogit <init|log|co|ci>"));
    }

    match args.nth(1).unwrap().as_str() {
        "init" => {
            git.init()?;
        }

        "log" => {
            let hist = git.log().context("log")?;
            for h in hist {
                println!("{} {}", h.hash.to_string(), h.msg)
            }
        }
        "ci" => {
            let mut buf = Vec::new();
            io::stdin().read_to_end(&mut buf).context("reading stdin")?;
            let parent = git.head().context("head")?;
            let msg = if let Some(msg) = args.nth(2) {
                msg
            } else {
                "fix".into()
            };

            git.add_commit("file.txt", buf.as_slice(), Some(parent), msg.as_str())?;
        }
        "co" => {
            let hash = args.nth(2).ok_or(anyhow!("expected commit hash"))?;

            let hist = git.log().context("log")?;
            for h in hist {
                if !h.hash.to_string().starts_with(hash.as_str()) {
                    continue;
                }
                let tree = git.tree(&h.tree.ok_or(anyhow!("no tree?"))?)?;
                for b in tree.blobs {
                    let content = git.blob(&b.hash)?;
                    println!("{}", String::from_utf8(content)?);
                }
                return Ok(());
            }
            return Err(anyhow!("unknown commit hash {hash}"));
        }
        cmd => return Err(anyhow!("unknown command: {cmd}",)),
    }

    Ok(())
}
