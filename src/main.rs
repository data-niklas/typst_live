use comemo::Prehashed;
use flate2::Compression;
use once_cell::unsync::OnceCell;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;
use std::pin::Pin;
use std::{io::Read, ptr::read};
use time::{Date, Month};
use typst::file::FileId;
use typst::geom::Color;

use js_sys::Array;
use wasm_bindgen::prelude::*;
use web_sys::Blob;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use typst::{
    diag::{FileError, FileResult},
    eval::Library,
    font::{Font, FontBook},
    syntax::Source,
    util::Bytes,
    World,
};

extern crate console_error_panic_hook;
use std::panic;

pub mod compat;
mod file;
pub mod lfs;
pub mod package;
use file::VFS;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

pub static MAIN_SOURCE_NAME: &'static str = "/main.typ";

fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn encode_string_into_url(text: &str) -> Option<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(text.as_bytes()).ok()?;
    let bytes = encoder.finish().ok()?;
    // hex::encode(bytes)
    Some(URL_SAFE_NO_PAD.encode(bytes))
}

#[wasm_bindgen]
pub fn decode_string_from_url(bytes: &str) -> Option<String> {
    let bytes = URL_SAFE_NO_PAD.decode(bytes).ok()?;
    let mut decoder = ZlibDecoder::new(&bytes[..]);
    let mut result = String::new();
    decoder.read_to_string(&mut result).ok()?;
    Some(result)
}

#[wasm_bindgen]
pub struct SystemWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    vfs: VFS,
}

impl SystemWorld {
    pub fn compile_to_pdf_bytes(&mut self, source: String) -> Result<Vec<u8>, JsValue> {
        self.vfs.set_main(source);
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

    pub fn compile_to_images_bytes(
        &mut self,
        source: String,
        pixel_per_pt: f32,
    ) -> Result<Vec<Vec<u8>>, JsValue> {
        self.vfs.set_main(source);
        match typst::compile(self) {
            Ok(document) => {
                let fill = Color::WHITE;
                let images = document
                    .pages
                    .into_iter()
                    .map(|page| typst::export::render(&page, pixel_per_pt, fill))
                    .map(|image| image.encode_png().expect("Could not encode as PNG"))
                    .collect();
                Ok(images)
            }
            Err(errors) => Err(errors
                .into_iter()
                .map(|error| JsValue::from_str(&error.message))
                .collect::<Array>()
                .into()),
        }
    }
}

#[wasm_bindgen]
impl SystemWorld {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SystemWorld {
        let mut searcher = FontSearcher::new();
        searcher.add_embedded();

        Self {
            library: Prehashed::new(typst_library::build()),
            book: Prehashed::new(searcher.book),
            fonts: searcher.fonts,
            vfs: VFS::new(),
        }
    }

    pub fn compile_to_pdf(&mut self, source: String) -> Result<String, JsValue> {
        let bytes = self.compile_to_pdf_bytes(source)?;
        let uint8arr = js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(&bytes) }.into());
        let array = js_sys::Array::new();
        array.push(&uint8arr.buffer());
        let blob = Blob::new_with_u8_array_sequence_and_options(
            &array,
            web_sys::BlobPropertyBag::new().type_("application/pdf"),
        )?;
        web_sys::Url::create_object_url_with_blob(&blob)
    }

    pub fn compile_to_images(
        &mut self,
        source: String,
        pixel_per_pt: f32,
    ) -> Result<js_sys::Array, JsValue> {
        let bytes = self.compile_to_images_bytes(source, pixel_per_pt)?;
        let urls = bytes
            .into_iter()
            .map(|bytes| {
                let uint8arr =
                    js_sys::Uint8Array::new(&unsafe { js_sys::Uint8Array::view(&bytes) }.into());
                let array = js_sys::Array::new();
                array.push(&uint8arr.buffer());
                let blob = Blob::new_with_u8_array_sequence_and_options(
                    &array,
                    web_sys::BlobPropertyBag::new().type_("image/png"),
                )
                .expect("Coult not create image BLOB's");
                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .expect("Could not create URL for BLOB");
                JsValue::from_str(&url)
            })
            .collect();
        Ok(urls)
    }
}

impl World for SystemWorld {
    fn packages(&self) -> &[(typst::file::PackageSpec, Option<typst::diag::EcoString>)] {
        &[]
    }

    fn today(&self, offset: Option<i64>) -> Option<typst::eval::Datetime> {
        let today = js_sys::Date::new_0();
        let year = today.get_full_year().try_into().expect("Not a year");
        let month = Month::try_from(today.get_month() as u8).expect("Not a month");
        let day = today.get_day().try_into().expect("Not a day");
        let today = Date::from_calendar_date(year, month, day).expect("Is not a valid date");
        Some(typst::eval::Datetime::Date(today))
    }

    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.vfs.get_main()
    }

    fn font(&self, id: usize) -> Option<Font> {
        let slot = &self.fonts[id];
        slot.font
            .get_or_init(|| Font::new(slot.buffer.clone(), slot.index))
            .clone()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.vfs.file(id)
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.vfs.source(id)
    }
}

/// Holds details about the location of a font and lazily the font itself.
struct FontSlot {
    buffer: Bytes,
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
            let buffer = Bytes::from_static(bytes);
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
