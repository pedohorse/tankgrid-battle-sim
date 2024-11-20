use std::sync::atomic::AtomicU64;

pub static NEXT_OBJID: AtomicU64 = AtomicU64::new(0);
