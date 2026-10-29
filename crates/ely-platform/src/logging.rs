use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    panic,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{Context, Result};
use env_logger::{Builder, Env, Target};

use crate::{AppId, data_dir};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

struct Tee(File);

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(buf)?;
        io::stderr().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        io::stderr().flush()
    }
}

/// The file logging writes to; none until `init` succeeds.
pub fn path() -> Option<&'static Path> {
    LOG_PATH.get().map(PathBuf::as_path)
}

/// Past this a log keeps only its newer half (§18.3).
const LOG_CAP: u64 = 8 * 1024 * 1024;

/// Keeps a long log's newer half, from a whole line.
fn trimmed(path: &Path, cap: u64) -> std::io::Result<()> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let length = file.metadata()?.len();
    if length <= cap {
        return Ok(());
    }
    let tail = log_tail(&mut file, length, cap)?;
    drop(file);
    fs::write(path, tail)
}

fn log_tail(file: &mut (impl Read + Seek), length: u64, cap: u64) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(length.saturating_sub(cap / 2)))?;
    let mut tail = Vec::new();
    file.take(cap / 2).read_to_end(&mut tail)?;
    let whole = tail.iter().position(|b| *b == b'\n').map_or(tail.len(), |n| n + 1);
    tail.drain(..whole);
    Ok(tail)
}

/// Logs to stderr and a file. `ELY_LOG` sets level.
pub fn init(app: AppId) -> Result<()> {
    init_in(app, &data_dir(app)?)
}

pub fn init_in(app: AppId, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = dir.join("ely.log");
    trimmed(&path, LOG_CAP).with_context(|| format!("cannot trim {}", path.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    Builder::from_env(Env::new().filter_or("ELY_LOG", "info"))
        .target(Target::Pipe(Box::new(Tee(file))))
        .try_init()
        .context("a logger is already set")?;
    LOG_PATH.set(path).expect("the logger was set once");

    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}");
        previous(info);
    }));
    log::info!("{} {} started", app.name(), env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod trim_tests {
    #[test]
    fn log_tail_reads_only_its_budget_even_when_the_log_is_large() {
        use std::io::{self, Read, Seek, SeekFrom};
        struct Source { length: u64, position: u64, read: usize }
        impl Read for Source {
            fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
                let count = bytes.len().min(self.length.saturating_sub(self.position) as usize);
                bytes[..count].fill(b'x');
                for (n, byte) in bytes[..count].iter_mut().enumerate() {
                    if (self.position + n as u64) % 16 == 15 { *byte = b'\n'; }
                }
                self.position += count as u64;
                self.read += count;
                Ok(count)
            }
        }
        impl Seek for Source {
            fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
                let SeekFrom::Start(at) = from else { panic!("only absolute seeks") };
                self.position = at;
                Ok(at)
            }
        }
        let mut source = Source { length: 1024 * 1024 * 1024, position: 0, read: 0 };
        let tail = super::log_tail(&mut source, 1024 * 1024 * 1024, 1024).expect("tail");
        assert_eq!(source.read, 512);
        assert_eq!(tail.len(), 496);
        assert!(tail.ends_with(b"\n"));
    }

    #[test]
    fn a_shortened_log_and_zero_budget_do_not_panic() {
        let mut source = std::io::Cursor::new(b"short\n");
        assert!(super::log_tail(&mut source, 1024, 16).expect("concurrent truncation").is_empty());
        assert!(super::log_tail(&mut source, 6, 0).expect("zero budget").is_empty());
    }

    #[test]
    fn a_long_log_keeps_its_newer_half_from_a_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ely.log");
        let lines: String = (0..200).map(|n| format!("line {n:03}\n")).collect();
        std::fs::write(&path, &lines).unwrap();
        super::trimmed(&path, 1000).unwrap();
        let kept = std::fs::read_to_string(&path).unwrap();
        assert!(kept.len() <= 500 && kept.starts_with("line ") && kept.ends_with("line 199\n"), "{kept}");
        super::trimmed(&path, 1000).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), kept, "a short log stays");
    }
}
