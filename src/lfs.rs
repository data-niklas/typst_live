


use web_sys::{window, Storage};

pub fn local_storage() -> Storage {
    window()
        .expect("Could not access window")
        .local_storage()
        .expect("Could not access local storage")
        .expect("Could not find a local storage")
}

pub struct LFS {
    storage: Storage,
}

impl LFS {
    pub fn new() -> Self {
        Self {
            storage: local_storage(),
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.storage
            .get(key)
            .expect("Could not use local storage")
            .is_some()
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.storage.get(key).expect("Could not use local storage")
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.get(key)
            .map(|text| hex::decode(text).expect("The file did not contain binary data"))
    }

    pub fn set(&self, key: &str, value: &str) {
        self.storage
            .set(key, value)
            .expect("Could not use local storage");
    }

    pub fn set_bytes(&self, key: &str, value: &[u8]) {
        self.set(key, &hex::encode(value));
    }

    pub fn delete(&self, key: &str) -> bool {
        if !self.exists(key) {
            return false;
        }
        self.storage
            .delete(key)
            .expect("Could not use local storage");
        true
    }

    pub fn list(&self) -> Vec<String> {
        let count: u32 = self
            .storage
            .length()
            .expect("Could not count items in local storage");
        (0..count)
            .into_iter()
            .map(|index| {
                self.storage
                    .key(index)
                    .expect("Could not read key from local storage")
                    .unwrap()
            })
            .collect()
    }
}
