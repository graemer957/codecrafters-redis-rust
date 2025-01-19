use std::collections::HashMap;
use std::sync::Mutex;

pub struct Store {
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        println!("getting value for {key:?}");
        self.inner.lock().ok()?.get(key).cloned()
    }

    pub fn set(&self, key: String, value: Vec<u8>) {
        if let Ok(value) = String::from_utf8(value.clone()) {
            println!("setting '{value}' for '{key}'");
        } else {
            println!("setting binary value for '{key}'");
        }

        if let Ok(mut lock) = self.inner.lock() {
            lock.insert(key, value);
        } else {
            eprintln!("Unable to acquire lock on Store");
        }
    }
}
