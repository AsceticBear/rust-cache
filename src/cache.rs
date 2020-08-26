use crossbeam::crossbeam_channel::tick;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_EXPIRATION: i64 = 0;

#[derive(Debug)]
pub struct Item {
    object: u64,
    expiration: SystemTime,
}

#[derive(Clone, Debug)]
pub struct RCache {
    cache: Arc<RwLock<Cache>>,
}

#[derive(Debug)]
pub struct Cache {
    default_expiration: i64,
    items: HashMap<String, Item>,
    janitor: Janitor,
}

#[derive(Debug)]
pub struct Janitor {
    interval: i64,
}

pub fn new(default_expiration: i64, clean_expiration: i64) -> RCache {
    let items = HashMap::new();
    return new_cache_with_janitor(default_expiration, clean_expiration, items);
}

fn new_cache_with_janitor(
    default_expiration: i64,
    clean_expiration: i64,
    items: HashMap<String, Item>,
) -> RCache {
    let c = new_cache(default_expiration, clean_expiration, items);
    let mut c_clone = c.clone();

    if clean_expiration > 0 {
        // start clean up janitor
        let _ = thread::spawn(move || {
            // set time to clean
            let ticker = tick(Duration::from_secs(clean_expiration as u64));
            loop {
                ticker.recv().unwrap();
                println!("janitor doing clean work");
                c_clone.delete_expired()
            }
        });
    }
    c
}
pub fn new_cache(
    default_expiration: i64,
    mut clean_expiration: i64,
    items: HashMap<String, Item>,
) -> RCache {
    if clean_expiration == DEFAULT_EXPIRATION {
        clean_expiration = -1;
    }

    let c = Cache {
        default_expiration,
        items,
        janitor: Janitor {
            interval: clean_expiration,
        },
    };

    return RCache {
        cache: Arc::new(RwLock::new(c)),
    };
}

impl Item {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now();

        if let Ok(passed) = now.duration_since(self.expiration) {
            if passed > Duration::from_secs(0) {
                return true;
            }
        }
        false
    }
}

impl RCache {
    pub fn set(&mut self, key: String, value: u64, mut ed: i64) {
        let c_lock = self.cache.clone();
        let mut c = c_lock.write().unwrap();

        if ed == DEFAULT_EXPIRATION {
            ed = c.default_expiration
        }

        let expiration_time = SystemTime::now().checked_add(Duration::from_secs(ed as u64));
        let i = Item {
            object: value,
            expiration: expiration_time.unwrap(),
        };
        c.items.insert(key, i);
    }

    pub fn set_with_default_exp(&mut self, key: String, value: u64) {
        self.set(key, value, DEFAULT_EXPIRATION)
    }

    pub fn flush(&mut self) {
        let c_lock = self.cache.clone();
        let mut c = c_lock.write().unwrap();

        c.items.clear()
    }

    pub fn delete_expired(&mut self) {
        let c_lock = self.cache.clone();
        let mut cw = c_lock.write().unwrap();

        let cr = c_lock.write().unwrap();
        for (key, item) in cr.items.iter() {
            if item.is_expired() {
                println!("janitor cleaned key {:?}", key);
                let _ = cw.items.remove_entry(key);
            }
        }
    }

    pub fn get(&self, key: String) -> Option<u64> {
        let c_lock = self.cache.clone();
        let c = c_lock.read().unwrap();

        if let Some(i) = c.items.get(&key) {
            if SystemTime::now() > i.expiration {
                return Some(0);
            }
            return Some(i.object);
        }
        Some(0)
    }

    pub fn item_count(&self) -> usize {
        let c_lock = self.cache.clone();
        let c = c_lock.read().unwrap();

        c.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache() {
        let mut tc = new(DEFAULT_EXPIRATION, 0);

        assert_eq!(tc.get("a".to_string()).unwrap(), 0);
        assert_eq!(tc.get("b".to_string()).unwrap(), 0);
        assert_eq!(tc.get("C".to_string()).unwrap(), 0);

        tc.set("a".to_string(), 1, DEFAULT_EXPIRATION);
        tc.set("b".to_string(), 2, DEFAULT_EXPIRATION);
        tc.set("C".to_string(), 3, DEFAULT_EXPIRATION);

        println!("cache is {:?}", tc);

        assert_eq!(tc.get("a".to_string()).unwrap(), 1);
        assert_eq!(tc.get("b".to_string()).unwrap(), 2);
        assert_eq!(tc.get("C".to_string()).unwrap(), 3);
    }
}
