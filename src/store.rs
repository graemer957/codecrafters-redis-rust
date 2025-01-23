use std::{
    collections::HashMap,
    str,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct Store {
    // TODO: Use `RwLock` instead?
    inner: Mutex<HashMap<String, Entry>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        println!("getting value for {key:?}");
        let mut lock = self.inner.lock().ok()?;

        match lock.get(key) {
            Some(entry)
                if entry.expires.is_some() && entry.expires.map(|ttl| ttl <= Instant::now())? =>
            {
                println!("removing value as expired...");
                lock.remove(key);

                None
            }
            Some(entry) => Some(entry.inner.clone()),
            None => None,
        }
    }

    pub fn set(&self, key: String, value: Vec<u8>, ttl: Option<Duration>) {
        if let Ok(value) = str::from_utf8(value.as_slice()) {
            println!("setting '{value}' for '{key}'");
        } else {
            println!("setting binary value for '{key}'");
        }

        match self.inner.lock() {
            Ok(mut lock) => {
                let entry = Entry::new(value, ttl);
                lock.insert(key, entry);
            }
            _ => {
                eprintln!("Unable to acquire lock on Store");
            }
        }
    }
}

struct Entry {
    inner: Vec<u8>,
    expires: Option<Instant>,
}

impl Entry {
    fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires = ttl.map(|ttl| Instant::now() + ttl);

        Self {
            inner: value,
            expires,
        }
    }
}
