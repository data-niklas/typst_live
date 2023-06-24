use comemo::Prehashed;
use elsa::FrozenVec;
use once_cell::unsync::OnceCell;
use std::path::Path;
use std::path::PathBuf;

use js_sys::Array;
use wasm_bindgen::prelude::*;
use web_sys::Blob;

use typst::{
    diag::{FileError, FileResult},
    eval::Library,
    font::{Font, FontBook},
    syntax::{Source, SourceId},
    util::Buffer,
    World,
};

fn main() {}

#[wasm_bindgen]
pub struct SystemWorld {
    root: PathBuf,
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    sources: FrozenVec<Box<Source>>,
    main: SourceId,
}

#[wasm_bindgen]
impl SystemWorld {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SystemWorld {
        let mut searcher = FontSearcher::new();
        searcher.add_embedded();

        Self {
            root: PathBuf::from("./"),
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            sources: FrozenVec::new(),
            main: SourceId::detached(),
        }
    }

    pub fn compile_to_pdf(&mut self, source: String) -> Result<String, JsValue> {
        let bytes = self.compile(source)?;
        let uint8arr = js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(&bytes) }.into());
        let array = js_sys::Array::new();
        array.push(&uint8arr.buffer());
        let blob = Blob::new_with_u8_array_sequence_and_options(
            &array,
            web_sys::BlobPropertyBag::new().type_("application/pdf"),
        )?;
        web_sys::Url::create_object_url_with_blob(&blob)
    }

    pub fn compile(&mut self, source: String) -> Result<Vec<u8>, JsValue> {
        self.sources.as_mut().clear();

        self.main = self.insert("<user input>".as_ref(), source);
        match typst::compile(self) {
            Ok(document) => {
                let render = typst::export::pdf(&document);
                Ok(render)
            }
            Err(errors) => Err(errors
                .into_iter()
                .map(|error| JsValue::from_str(&error.message))
                .collect::<Array>()
                .into()),
        }
    }
}

impl World for SystemWorld {
    fn root(&self) -> &Path {
        &self.root
    }

    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn main(&self) -> &Source {
        self.source(self.main)
    }

    fn resolve(&self, _path: &Path) -> FileResult<SourceId> {
        Err(FileError::AccessDenied)
    }

    fn source(&self, id: SourceId) -> &Source {
        &self.sources[id.into_u16() as usize]
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn font(&self, id: usize) -> Option<Font> {
        let slot = &self.fonts[id];
        slot.font
            .get_or_init(|| Font::new(slot.buffer.clone(), slot.index))
            .clone()
    }

    fn file(&self, _path: &Path) -> FileResult<Buffer> {
        Err(FileError::AccessDenied)
    }
}

impl SystemWorld {
    fn insert(&self, path: &Path, text: String) -> SourceId {
        let id = SourceId::from_u16(self.sources.len() as u16);
        let source = Source::new(id, path, text);
        self.sources.push(Box::new(source));
        id
    }
}

/// Holds details about the location of a font and lazily the font itself.
struct FontSlot {
    buffer: Buffer,
    index: u32,
    font: OnceCell<Option<Font>>,
}

struct FontSearcher {
    book: FontBook,
    fonts: Vec<FontSlot>,
}

impl FontSearcher {
    fn new() -> Self {
        Self {
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    fn add_embedded(&mut self) {
        let mut add = |bytes: &'static [u8]| {
            let buffer = Buffer::from_static(bytes);
            for (i, font) in Font::iter(buffer.clone()).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    buffer: buffer.clone(),
                    index: i as u32,
                    font: OnceCell::from(Some(font)),
                });
            }
        };

        // Embed default fonts.
        add(include_bytes!("../assets/fonts/LinLibertine_R.ttf"));
        add(include_bytes!("../assets/fonts/LinLibertine_RB.ttf"));
        add(include_bytes!("../assets/fonts/LinLibertine_RBI.ttf"));
        add(include_bytes!("../assets/fonts/LinLibertine_RI.ttf"));
        add(include_bytes!("../assets/fonts/NewCMMath-Book.otf"));
        add(include_bytes!("../assets/fonts/NewCMMath-Regular.otf"));
        add(include_bytes!("../assets/fonts/DejaVuSansMono.ttf"));
        add(include_bytes!("../assets/fonts/DejaVuSansMono-Bold.ttf"));
        add(include_bytes!("../assets/fonts/DejaVuSansMono-Oblique.ttf"));
        add(include_bytes!(
            "../assets/fonts/DejaVuSansMono-BoldOblique.ttf"
        ));
    }
}
