use std::fmt::Display;
use typst::syntax::{PackageSpec, PackageVersion as Version};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub struct WasmVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl From<Version> for WasmVersion {
    fn from(value: Version) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
        }
    }
}

#[wasm_bindgen]
impl WasmVersion {
    #[wasm_bindgen(constructor)]
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl Display for WasmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}.{}.{}", self.major, self.minor, self.patch))
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct WasmPackageSpec {
    #[wasm_bindgen(skip)]
    pub namespace: String,
    #[wasm_bindgen(skip)]
    pub name: String,
    pub version: WasmVersion,
}

impl From<PackageSpec> for WasmPackageSpec {
    fn from(value: PackageSpec) -> Self {
        Self {
            namespace: value.namespace.into(),
            name: value.name.into(),
            version: value.version.into(),
        }
    }
}

#[wasm_bindgen]
impl WasmPackageSpec {
    #[wasm_bindgen(constructor)]
    pub fn new(namespace: String, name: String, version: WasmVersion) -> Self {
        Self {
            namespace,
            name,
            version,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn namespace(&self) -> String {
        self.namespace.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    #[wasm_bindgen(setter)]
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_owned()
    }
    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    pub fn package_directory(&self) -> String {
        format!("packages/{}/{}/{}", self.namespace, self.name, self.version)
    }
    pub fn package_directory_key(&self) -> String {
        format!(
            "packages/{}/{}/{}/.",
            self.namespace, self.name, self.version
        )
    }
}
