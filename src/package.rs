use crate::compat::{WasmPackageSpec, WasmVersion};
use crate::lfs::LFS;
use js_sys::{Array, ArrayBuffer, JsString, Promise, Uint8Array};
use regex::Regex;
use std::io::Write;
use std::marker::Copy;
use std::str::FromStr;
use std::string::String;
use std::sync::Arc;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use typst::diag::EcoString;
use typst::{
    diag::{PackageError, PackageResult},
    syntax::{PackageSpec, PackageVersion as Version},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::console;
use web_sys::{Request, RequestInit, RequestMode, Response};

pub fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
    if spec.namespace != "preview" {
        return PackageResult::Err(PackageError::Other(None));
    }
    let subdir = format!("packages/{}/{}/{}", spec.namespace, spec.name, spec.version);
    let subdir_key = subdir.clone() + "/.";
    if !LFS::new().exists(&subdir_key) {
        console::log_1(&"Package does not exist".into());
        return PackageResult::Err(PackageError::NotFound(spec.clone()));
    }
    PackageResult::Ok(Path::new(&subdir).to_owned())
}

#[wasm_bindgen]
pub struct PackageManager {
    lfs: Arc<LFS>,
}

const PACKAGE_PATH_SPEC_REGEX: &str = r"packages/([^/]+)/([^-]+)/([0-9]+).([0-9]+).([0-9]+)";
const PACKAGE_SPEC_REGEX: &str = r"@([^/]+)/([^-]+):([0-9]+).([0-9]+).([0-9]+)";

#[wasm_bindgen]
impl PackageManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            lfs: Arc::new(LFS::new()),
        }
    }

    pub fn list_packages(&self) -> Array {
        let keys: Vec<String> = self.lfs.list();
        let installed_package_paths: Vec<String> = keys
            .into_iter()
            .filter(|key| key.starts_with("packages/") && key.ends_with("/."))
            .collect();
        let packages: Vec<WasmPackageSpec> = installed_package_paths
            .into_iter()
            .map(|path: String| {
                let captures_regex = Regex::new(PACKAGE_PATH_SPEC_REGEX).unwrap();
                let captures = captures_regex.captures(&path).unwrap();
                let namespace = captures[1].to_owned();
                let name = captures[2].to_owned();
                let major = captures[3].parse().expect("");
                let minor = captures[4].parse().expect("");
                let patch = captures[5].parse().expect("");
                let version = WasmVersion::new(major, minor, patch);
                WasmPackageSpec::new(namespace, name, version)
            })
            .collect();
        packages.into_iter().map(JsValue::from).collect()
    }

    pub fn delete_package(&self, pkg: WasmPackageSpec) {
        let package_path = format!(
            "packages/{}/{}-{}.{}.{}",
            pkg.namespace, pkg.name, pkg.version.major, pkg.version.minor, pkg.version.patch
        );
        let keys: Vec<String> = self.lfs.list();
        let package_keys: Vec<String> = keys
            .into_iter()
            .filter(|key| key.starts_with(&package_path))
            .collect();
        for key in package_keys {
            self.lfs.delete(&key);
        }
    }

    pub fn download_package_from_str(&self, spec: &str) -> Promise {
        let captures_regex = Regex::new(PACKAGE_SPEC_REGEX).unwrap();
        let captures = captures_regex.captures(&spec).unwrap();
        let namespace = captures[1].to_owned();
        let name = captures[2].to_owned();
        let major = captures[3].parse().expect("");
        let minor = captures[4].parse().expect("");
        let patch = captures[5].parse().expect("");
        let version = WasmVersion::new(major, minor, patch);
        let spec = WasmPackageSpec::new(namespace, name, version);
        self.download_package(&spec)
    }

    pub fn download_package(&self, spec: &WasmPackageSpec) -> Promise {
        // The `@preview` namespace is the only namespace that supports on-demand
        // fetching.
        if self.lfs.exists(&spec.package_directory_key()) {
            return Promise::resolve(&JsValue::from_str("The package already exists"));
        }
        let package_dir = spec.package_directory();
        let package_dir = Path::new(&package_dir);
        assert_eq!(spec.namespace, "preview");

        let closure_package_dir = package_dir.to_owned();

        let url = format!(
            "https://packages.typst.org/preview/{}-{}.tar.gz",
            spec.name, spec.version
        );
        let mut opts = RequestInit::new();
        opts.method("GET");
        opts.mode(RequestMode::Cors);
        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|_| PackageError::Other(None))
            .expect("Could not send request");
        let lfs = self.lfs.clone();

        return Promise::new(&mut move |resolve, reject| {
            let closure_package_dir = closure_package_dir.clone();
            let lfs = self.lfs.clone();
            let window = web_sys::window().expect("Could not get window");
            let dl_promise = window.fetch_with_request(&request);
            let dl_closure = Closure::<dyn FnMut(JsValue)>::new(move |resp_value: JsValue| {
                let closure_package_dir = closure_package_dir.clone();
                let lfs = lfs.clone();
                assert!(resp_value.is_instance_of::<Response>());
                let resp: Response = resp_value.dyn_into().unwrap();
                let resolve = resolve.clone();
                let content_promise = resp.array_buffer().expect("Should have a body");
                let content_closure =
                    Closure::<dyn FnMut(JsValue)>::new(move |content: JsValue| {
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
                                let file_path =
                                    file_path.to_str().expect("Could not convert Path to str");
                                let bytes: Vec<u8> = entry
                                    .bytes()
                                    .into_iter()
                                    .map(|byte| byte.expect("Could not read byte"))
                                    .collect();
                                lfs.set_bytes(file_path, &bytes[..]);
                            });
                        resolve.call0(&JsValue::null());
                    });
                content_promise.then(&content_closure);
                content_closure.forget();
            });
            dl_promise.then(&dl_closure);
            dl_closure.forget();
        });
        // return Promise::resolve(&JsValue::null());
    }
}
