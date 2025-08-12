
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

pub type TaskId = usize;

pub fn spawn<F>(f: F) -> TaskId 
where F: FnOnce() + Send + 'static {
    static mut NEXT: usize = 1;
    let id;
    unsafe { id = NEXT; NEXT += 1; }
    std::thread::spawn(f);
    id
}

pub fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

pub fn join(_id: TaskId) {
    // naive placeholder; in real runtime we track JoinHandle
}

pub fn version()->&'static str { "async-mini 0.1.0" }
