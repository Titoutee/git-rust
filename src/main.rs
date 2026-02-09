use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::{Compression, write};
use flate2::{read::ZlibDecoder, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use std::ffi::CStr;
use std::fs;
// use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod hash;
use hash::HashWriter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

/// Doc comment
#[derive(Debug, Subcommand)]
enum Command {
    /// Doc comment
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write_dir: bool,

        file: PathBuf,
    },
}

#[derive(Debug)]
enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir_all("res/.git/objects").unwrap();
            fs::create_dir_all("res/.git/refs").unwrap();
            fs::write("res/.git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print, // TODO: make pretty print useful
            object_hash,
        } => {
            // TODO: support shortest-unique object hashes
            let f = std::fs::File::open(format!(
                ".git/objects/{}/{}",
                &object_hash[..2], // 2-digit hash prefix for subdirectory
                &object_hash[2..]  // rest of the hash for the file itself
            ))
            .context("open in .git/objects")?;
            let z = ZlibDecoder::new(f);
            let mut z = BufReader::new(z);
            let mut buf = Vec::new();
            z.read_until(0, &mut buf)
                .context("read header from .git/objects")?; // read kind and size
            let header = CStr::from_bytes_with_nul(&buf)
                .expect("know there is exactly one nul, and it's at the end");
            let header = header
                .to_str()
                .context(".git/objects file header isn't valid UTF-8")?;
            let Some((kind, size)) = header.split_once(' ') else {
                anyhow::bail!(
                    ".git/objects file header did not start with known bytes: '{header}'"
                );
            };
            let kind = match kind {
                "blob" => Kind::Blob,
                _ => {
                    anyhow::bail!(format!("unknown kind: {}", kind))
                }
            };
            let size = size
                .parse::<usize>()
                .context(".git/objects file header has invalid size: {size}")?;
            buf.clear(); // Recycling...
            buf.resize(size, 0);
            z.read_exact(&mut buf[..])
                .context("read true contents of .git/objects file")?;

            let n = z
                .read(&mut [0])
                .context("validate EOF in .git/object file")?;
            anyhow::ensure!(n == 0, ".git/object file had {n} trailing bytes");

            let mut stdout = std::io::stdout().lock();

            match kind {
                Kind::Blob => stdout
                    .write_all(&buf)
                    .context("write object contents to stdout")?,
            }
        }
        Command::HashObject { write_dir, file } => {
            fn write_blob<W: Write>(file: &Path, writer: W) -> anyhow::Result<String> {
                let stat = std::fs::metadata(file)?;
                let writer = ZlibEncoder::new(writer, Compression::best());
                let mut writer = HashWriter {
                    writer,
                    hasher: Sha1::new(),
                };

                write!(writer, "blob ")?;
                write!(writer, "{}\0", stat.len())?;

                let mut file = std::fs::File::open(&file)
                    .with_context(|| format!("open {}", file.display()))?;
                std::io::copy(&mut file, &mut writer).context("stream file into blob")?;

                let _ = writer.writer.finish()?;
                let hash = writer.hasher.finalize();
                Ok(hex::encode(hash))
            }

            let hash = if write_dir {
                String::new() // TODO
            } else {
                write_blob(&file, std::io::sink())?
            };
        }
    }

    Ok(())
}
