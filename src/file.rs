use crate::{SystemWorld, MAIN_SOURCE_NAME};
use once_cell::unsync::OnceCell;
use siphasher::sip128::{Hasher128, SipHasher13};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::io::Read;
use std::path::{Path, PathBuf};
use typst::file::PackageSpec;
use typst::util::PathExt;
use typst::{
    diag::{FileError, FileResult},
    file::FileId,
    syntax::Source,
    util::Bytes,
    World,
};
use wasm_bindgen::JsValue;

use crate::lfs::LFS;
use crate::package::prepare_package;

pub struct VFS {
    main: Option<Source>,
    main_id: FileId,
    hashes: RefCell<HashMap<FileId, FileResult<PathHash>>>,
    paths: RefCell<HashMap<PathHash, PathSlot>>,
}

impl VFS {
    pub fn new() -> Self {
        Self {
            main: None,
            main_id: FileId::new(None, Path::new(MAIN_SOURCE_NAME)),
            hashes: RefCell::default(),
            paths: RefCell::default(),
        }
    }
    pub fn source(&self, id: FileId) -> Result<Source, FileError> {
        if id.path().to_str().unwrap() == MAIN_SOURCE_NAME {
            Ok(self.get_main())
        } else {
            self.slot(id)?.source()
        }
    }
    pub fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id.path().to_str().unwrap() == MAIN_SOURCE_NAME {
            let main_source = self.main.as_ref().unwrap();
            let main_text = main_source.text();
            let main_bytes = main_text.as_bytes();
            Ok(Bytes::from(main_bytes))
        } else {
            self.slot(id)?.file()
        }
    }

    pub fn set_main(&mut self, source: String) {
        self.main = Some(Source::new(self.main_id, source));
    }

    pub fn get_main(&self) -> Source {
        self.main.clone().expect("No main was set!")
    }

    fn slot(&self, id: FileId) -> FileResult<RefMut<PathSlot>> {
        let mut system_path = PathBuf::new();
        let error_hash = self
            .hashes
            .borrow_mut()
            .get(&id)
            .map_or(false, |value| value.is_err());
        if error_hash {
            let root = match id.package() {
                Some(spec) => prepare_package(spec)?,
                None => Path::new("/").to_owned(),
            };
            system_path = root.join_rooted(id.path()).ok_or(FileError::AccessDenied)?;
            let hash = PathHash::new(&system_path);

            self.hashes.borrow_mut().insert(id, hash);
        }
        let hash = self
            .hashes
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                // Determine the root path relative to which the file path
                // will be resolved.
                let root = match id.package() {
                    Some(spec) => prepare_package(spec)?,
                    None => Path::new("/").to_owned(),
                };

                // Join the path to the root. If it tries to escape, deny
                // access. Note: It can still escape via symlinks.
                system_path = root.join_rooted(id.path()).ok_or(FileError::AccessDenied)?;
                PathHash::new(&system_path)
            })
            .clone()?;

        Ok(RefMut::map(self.paths.borrow_mut(), |paths| {
            paths.entry(hash).or_insert_with(|| PathSlot {
                id,
                // This will only trigger if the `or_insert_with` above also
                // triggered.
                system_path,
                source: OnceCell::new(),
                buffer: OnceCell::new(),
            })
        }))
    }
}

struct PathSlot {
    /// The slot's canonical file id.
    id: FileId,
    /// The slot's path on the system.
    system_path: PathBuf,
    /// The lazily loaded source file for a path hash.
    source: OnceCell<FileResult<Source>>,
    /// The lazily loaded buffer for a path hash.
    buffer: OnceCell<FileResult<Bytes>>,
}

impl PathSlot {
    fn source(&mut self) -> FileResult<Source> {
        if let Some(Err(_)) = self.source.get() {
            self.source = OnceCell::new();
        }
        self.source
            .get_or_init(|| {
                let buf = read(&self.system_path)?;
                let text = decode_utf8(buf)?;
                Ok(Source::new(self.id, text))
            })
            .clone()
    }

    fn file(&mut self) -> FileResult<Bytes> {
        if let Some(Err(_)) = self.buffer.get() {
            self.buffer = OnceCell::new();
        }
        self.buffer
            .get_or_init(|| read(&self.system_path).map(Bytes::from))
            .clone()
    }
}

/// A hash that is the same for all paths pointing to the same entity.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct PathHash(u128);

impl PathHash {
    fn new(path: &Path) -> FileResult<Self> {
        let f = |e| FileError::from_io(e, path);
        let mut state = SipHasher13::new();
        path.to_str().unwrap().hash(&mut state);
        let content = read(path)?;
        content.hash(&mut state);
        Ok(Self(state.finish128().as_u128()))
    }
}

/// Read a file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    let lfs = LFS::new();
    let key = path.to_str().ok_or(FileError::Other)?;
    if !lfs.exists(key) {
        return Err(FileError::NotFound(path.to_owned()));
    }
    lfs.get_bytes(key).ok_or(FileError::Other)
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: Vec<u8>) -> FileResult<String> {
    Ok(if buf.starts_with(b"\xef\xbb\xbf") {
        // Remove UTF-8 BOM.
        std::str::from_utf8(&buf[3..])?.into()
    } else {
        // Assume UTF-8.
        String::from_utf8(buf)?
    })
}
