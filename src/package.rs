use js_sys::{ArrayBuffer, Uint8Array};
use localstoragefs::fs::File;
use std::io::Write;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use typst::diag::EcoString;
use typst::{
    diag::{PackageError, PackageResult},
    file::PackageSpec,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::console;
use web_sys::{Request, RequestInit, RequestMode, Response};
// use std::sync::mpsc::channel;
pub struct PackageManager {}

impl PackageManager {
    pub fn new() -> Self {
        let it = Self {};
        let spec = PackageSpec {
            namespace: "preview".into(),
            name: "syntree".into(),
            version: typst::file::Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
        };
        it.prepare_package(&spec);
        it
    }
    fn package_exists(spec: &PackageSpec) -> bool {
        let toml_file = format!(
            "packages/{}/{}-{}/typst.toml",
            spec.namespace, spec.name, spec.version
        );
        let toml_path = Path::new(&toml_file);
        match File::open(toml_path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn prepare_package(&self, spec: &PackageSpec) -> PackageResult<PathBuf> {
        if spec.namespace != "preview" {
            return Err(PackageError::Other);
        }
        let subdir = format!("packages/{}/{}-{}", spec.namespace, spec.name, spec.version);

        if !Self::package_exists(spec) {
            console::log_1(&"Package does not exist".into());
            self.download_package(spec, Path::new(&subdir))?
        }
        Ok(Path::new(&subdir).to_owned())
    }

    // fn download_sync(&self, url: String) -> PackageResult<Vec<u8>> {
    //     // let future_response = JsFuture::from(window.fetch_with_request(&request));
    //     // let (tx, mut rx) = mpsc::channel(1);
    //     // executor::spawn(async move {
    //     // let resp_value = future_response.await.expect("");
    //     // assert!(resp_value.is_instance_of::<Response>());
    //     // let resp: Response = resp_value.dyn_into().unwrap();
    //     // let content_promise = resp.array_buffer().expect("Should have a body");
    //     // let content = JsFuture::from(content_promise).await.expect("Could not get response");
    //     // let rusty_content = Uint8Array::new(&content).to_vec();
    //     // tx.send(rusty_content);
    //     // });
    //     // Ok(rx.blocking_recv().expect("Could not unwrap option"))
    // }

    /// Download a package over the network.
    fn download_package(&self, spec: &PackageSpec, package_dir: &Path) -> PackageResult<()> {
        // The `@preview` namespace is the only namespace that supports on-demand
        // fetching.
        assert_eq!(spec.namespace, "preview");

        let closure_package_dir = package_dir.to_owned();

        let url = format!(
            "https://packages.typst.org/preview/{}-{}.tar.gz",
            spec.name, spec.version
        );
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        let request =
            Request::new_with_str_and_init(&url, &opts).map_err(|_| PackageError::Other)?;
        let window = web_sys::window().expect("Could not get window");
        let dl_promise = window.fetch_with_request(&request);
        let dl_closure = Closure::<dyn FnMut(JsValue)>::new(move |resp_value: JsValue| {
            let closure_package_dir = closure_package_dir.clone();
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();
            let content_promise = resp.array_buffer().expect("Should have a body");
            let content_closure = Closure::<dyn FnMut(JsValue)>::new(move |content: JsValue| {
                let content: ArrayBuffer = content.dyn_into().unwrap();
                let rusty_content = Uint8Array::new(&content).to_vec();
                let decompressed = flate2::read::GzDecoder::new(&rusty_content[..]);
                let mut archive = tar::Archive::new(decompressed);
                archive
                    .entries()
                    .expect("Could not read entries")
                    .into_iter()
                    .for_each(|entry| {
                        let entry = entry.expect("Could not read entry");
                        let path = entry.path().expect("Could not read path");
                        let package_dir = closure_package_dir.clone();
                        let file_path = package_dir.join(path);
                        let mut file =
                            File::create(file_path.clone()).expect("Could not create file");
                        let bytes: Vec<u8> = entry
                            .bytes()
                            .into_iter()
                            .map(|byte| byte.expect("Could not read byte"))
                            .collect();
                        file.write_all(&bytes[..]);
                    });
            });
            content_promise.then(&content_closure);
            content_closure.forget();
        });
        dl_promise.then(&dl_closure);
        dl_closure.forget();
        Ok(())
    }
}
