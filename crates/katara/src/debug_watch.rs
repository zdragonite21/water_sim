#![allow(unused)]
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

struct Watch {
    map: BTreeMap<&'static str, WatchEntry>,
    frame: u64,
}

static WATCHES: LazyLock<Mutex<Watch>> = LazyLock::new(|| {
    Mutex::new(Watch {
        map: BTreeMap::new(),
        frame: 0,
    })
});

#[derive(Clone)]
pub struct WatchEntry {
    pub value: String,
    pub file: &'static str,
    pub line: u32,
    pub last_seen_frame: u64,
}

pub fn set(name: &'static str, value: impl Into<String>, file: &'static str, line: u32) {
    let mut watch = WATCHES.lock().unwrap();
    let frame = watch.frame;

    watch.map.insert(
        name,
        WatchEntry {
            value: value.into(),
            file,
            line,
            last_seen_frame: frame,
        },
    );
}

pub fn snapshot() -> Vec<(&'static str, WatchEntry)> {
    WATCHES
        .lock()
        .unwrap()
        .map
        .iter()
        .map(|(name, entry)| (*name, entry.clone()))
        .collect()
}

pub fn begin_frame(frame: u64, paused: bool) {
    let mut watch = WATCHES.lock().unwrap();
    watch.frame = frame;
    watch.map.retain(|_, entry| entry.last_seen_frame == frame || paused);
}

pub fn clear() {
    WATCHES.lock().unwrap().map.clear(); 
}

#[macro_export]
macro_rules! dwatch {
    ($name:literal, $fmt:literal, $($arg:tt)*) => {
        $crate::debug_watch::set(
            $name,
            format!($fmt, $($arg)*),
            file!(),
            line!(),
        )
    };

    ($name:literal, $value:expr) => {
        $crate::debug_watch::set(
            $name,
            format!("{}", $value),
            file!(),
            line!(),
        )
    };
}
