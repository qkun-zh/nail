use gloo_storage::{LocalStorage, Storage};

pub fn read(key: &str) -> Option<String> {
    LocalStorage::get::<String>(key).ok()
}

pub fn write(key: &str, value: &str) {
    let _ = LocalStorage::set(key, value);
}

pub fn remove(key: &str) {
    LocalStorage::delete(key);
}
