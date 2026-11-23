//! The one reader of a file that a session can write (#261): a file under `.watchbill/` of the working
//! tree, of a view, or of the state folder. What is not a plain file is never read, and what is read is read
//! through a cap, so that no file makes `wb` wait or fill the memory.

use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// The most a file of text (`watchbill.yml`, a part of the project's text, an issue's file) is read.
pub const TEXT_CAP: u64 = 1024 * 1024;

/// The most a file of the revision is read.
pub const REVISION_CAP: u64 = 16 * 1024 * 1024;

/// Why a file was not read.
#[derive(Debug)]
pub enum Fault {
    /// There is nothing at the path.
    NotFound,
    /// A FIFO, a device, a folder or a link: never opened as a file to read.
    NotPlain,
    /// More than the cap, which is held here.
    TooBig(u64),
    /// The system would not open or read it.
    Io(io::Error),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::NotFound => f.write_str("it is not there"),
            Fault::NotPlain => f.write_str("it is not a plain file"),
            Fault::TooBig(cap) if cap % (1024 * 1024) == 0 => write!(f, "it is over {} MiB", cap / (1024 * 1024)),
            Fault::TooBig(cap) => write!(f, "it is over {} bytes", cap),
            Fault::Io(error) => write!(f, "{}", crate::yamlfile::system_words(error)),
        }
    }
}

impl Fault {
    /// The fault as an error of the system, for a caller that carries one.
    pub fn into_io(self) -> io::Error {
        match self {
            Fault::Io(error) => error,
            Fault::NotFound => io::Error::new(io::ErrorKind::NotFound, self.to_string()),
            _ => io::Error::new(io::ErrorKind::Other, self.to_string()),
        }
    }
}

/// The text of a plain file of `cap` bytes or fewer, or the fault.
pub fn read_text(path: &Path, cap: u64) -> Result<String, Fault> {
    String::from_utf8(read_plain(path, cap)?).map_err(|_| Fault::Io(io::Error::new(io::ErrorKind::InvalidData, "it is not UTF-8 text")))
}

impl From<io::Error> for Fault {
    fn from(error: io::Error) -> Fault {
        if error.kind() == io::ErrorKind::NotFound {
            Fault::NotFound
        } else {
            Fault::Io(error)
        }
    }
}

/// Opens a plain file to read. The path is looked at first and is never followed as a link. It is opened without
/// waiting (a FIFO opens at once), and it is the open file that is looked at again, so a file swapped for a
/// FIFO between the look and the open does not make `wb` wait. The metadata is that of the open file.
pub fn open_plain(path: &Path) -> Result<(File, fs::Metadata), Fault> {
    open(path, false)
}

fn open(path: &Path, follow: bool) -> Result<(File, fs::Metadata), Fault> {
    let looked = if follow { fs::metadata(path)? } else { fs::symlink_metadata(path)? };
    if !looked.file_type().is_file() {
        return Err(Fault::NotPlain);
    }
    let file = fs::OpenOptions::new().read(true).custom_flags(crate::manifest::O_NONBLOCK).open(path)?;
    let opened = file.metadata()?;
    if !opened.file_type().is_file() {
        return Err(Fault::NotPlain);
    }
    Ok((file, opened))
}

/// The bytes of a plain file of `cap` bytes or fewer. A larger file is not read past `cap` and one byte.
pub fn read_plain(path: &Path, cap: u64) -> Result<Vec<u8>, Fault> {
    read_from(open(path, false)?, cap)
}

/// As `read_plain`, for a file outside the working tree that a session cannot write (the organisation's file, the
/// installed tables), which is read where it is kept as a link: the file the link names must be a plain file.
pub fn read_following(path: &Path, cap: u64) -> Result<Vec<u8>, Fault> {
    read_from(open(path, true)?, cap)
}

fn read_from((file, opened): (File, fs::Metadata), cap: u64) -> Result<Vec<u8>, Fault> {
    if opened.len() > cap {
        return Err(Fault::TooBig(cap));
    }
    let mut bytes = Vec::new();
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(Fault::TooBig(cap));
    }
    Ok(bytes)
}

/// Whether there is nothing at the path (a link counts as something).
pub fn is_absent(path: &Path) -> bool {
    matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}
