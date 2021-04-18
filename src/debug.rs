
use std::{cell::{Cell, RefCell}, sync::{Mutex, MutexGuard, Once, mpsc::{ channel, Sender, Receiver }}, thread::{self, JoinHandle, ThreadId}, time::Instant};

pub trait MemoryUse {
    fn memory_use_estimate(&self) -> usize;
}

#[derive(Debug, Clone)]
pub enum LogMessageContents {
    Log(String),
    Warn(String),
    Error(String),
    Debug(String),
    Fatal(String),
    Close,
}

#[derive(Debug)]
pub struct LogMessage {
    thread: ThreadId,
    time: Instant,
    module: &'static str,
    contents: LogMessageContents,
}

impl LogMessage {
    pub fn new(module: &'static str, contents: LogMessageContents) -> Self {
        LogMessage {
            time: ::std::time::Instant::now(),
            thread: ::std::thread::current().id(),
            module: module,
            contents: contents,
        }
    }
}

struct LogRx(Cell<Option<Mutex<Receiver<LogMessage>>>>);
impl LogRx {
    const fn new() -> Self {
        LogRx(Cell::new(None))
    }
}

struct LogTx(Cell<Option<Mutex<Sender<LogMessage>>>>);

impl LogTx {
    const fn new() -> Self {
        LogTx(Cell::new(None))
    }
}

static mut LOG_INIT_ONCE: Once = Once::new();
static mut LOG_SENDER_MUTEX: LogTx = LogTx::new();
static mut LOG_RECEIVER_MUTEX: LogRx = LogRx::new();
static mut LOG_SINK_JOIN_HANDLE: Cell<Option<JoinHandle<RecvGuardTunnel>>> = Cell::new(None);

// Warning: Implementing 'Send' for a 'MutexGuard' is unsafe and can lead to undefined behavior.
// 
//          Don't do this without being aware of the risks
//
// This is used to keep the receiver mutex locked while the log sink thread spawns.
// It's important that the mutex isn't unlocked and then re-locked during thread creation
// in case the log channel is cleaned up before the log sink initializes. We need to maintain
// the lock for the entire program duration.
struct RecvGuardTunnel<'a>(MutexGuard<'a, Receiver<LogMessage>>);
unsafe impl<'a> Send for RecvGuardTunnel<'a> {}

pub unsafe fn get_log_channel() -> Sender<LogMessage> {
    LOG_INIT_ONCE.call_once(|| {
        let (tx, rx) = channel::<LogMessage>();

        LOG_SENDER_MUTEX.0.set(Some(Mutex::new(tx)));
        LOG_RECEIVER_MUTEX.0.set(Some(Mutex::new(rx)));
        
        // Acquire the rx lock, store the MutexGuard in a RecvGuardTunnel so it
        // can be passed directly to the log sink thread without unlocking it
        // 
        // Safety:
        // 
        // Typically passing a MutexGuard between threads is unsafe and can result in undefined behavior.
        // In this case, we ensure that we do not mutate the guard contents in this thread - we just want
        // to hold the lock across a single thread boundary. Additionally, we still pass ownership of the
        // guard into the sink thread. The sink thread then returns ownership of the guard when it's joined
        // and finally we drop the guard in this original thread. 
        let guard_tunnel = match *LOG_RECEIVER_MUTEX.0.as_ptr() {
            Some(ref receiver_guard) => {
                match receiver_guard.lock() {
                    Ok(guard) => {
                        RecvGuardTunnel(guard)   
                    },
                    Err(e) => {
                        panic!("Failed to acquire log channel receiver guard: {:?}", e);
                    }
                }
            },
            None => {
                panic!("Attempted to reference uninitialized receiver during log channel creation");
            }
        };

        // Spawn the log sink thread
        let handle = thread::spawn(move || {
            let receiver_guard = guard_tunnel.0;

            // receiver loop
            loop {
                let mut close = false;
                match receiver_guard.recv() {
                    Ok(msg) => {
                        match msg.contents {
                            LogMessageContents::Close => {
                                close = true;
                                sink(msg);
                            },
                            _ => {
                                sink(msg);
                            }
                        }
                    },
                    Err(e) => {
                        println!("Log channel recv failed: {:?}", e);
                        return RecvGuardTunnel(receiver_guard)
                    }
                }

                if close {
                    println!("Closing log sink thread");
                    return RecvGuardTunnel(receiver_guard)
                }
            }
        });
        
        LOG_SINK_JOIN_HANDLE.set(Some(handle));
    }); // end of LOG_INIT_ONCE

    // get_log_channel logic
    match *LOG_SENDER_MUTEX.0.as_ptr() {
        Some(ref log_tx) => {
            match log_tx.lock() {
                Ok(log_tx) => {
                    return log_tx.clone()
                },
                Err(e) => {
                    panic!("Failed to acquire log channel sender guard: {:?}", e);
                }
            }
        },
        None => {
            panic!("Attempt to reference uninitialized log channel");
        }
    }
}

pub unsafe fn cleanup_log_channel() {
    let _ = get_log_channel().send(LogMessage {
        time: Instant::now(),
        thread: thread::current().id(),
        module: module_path!(),
        contents: LogMessageContents::Close,
    });

    match LOG_SINK_JOIN_HANDLE.take() {
        Some(handle) => {
            match handle.join() {
                Ok(guard_tunnel) => {
                    std::mem::drop(guard_tunnel);
                },
                Err(e) => {
                    let str = e
                        .downcast_ref::<&'static str>().map(|s| String::from(*s))
                        .or(e.downcast_ref::<String>().map(|s| String::from(s)))
                        .or(Some(String::from(format!("{:?}", e)))).unwrap();
                    
                    panic!("Failed to join log sink thread: {:?}", str);
                }
            }
        },
        None => {
            panic!("Attempt to join non-existent log sink thread");
        }
    }
}

fn sink(msg: LogMessage) {
    // Todo: - Write messages to some structure, periodically dump data into the output
    //       - Ensure all data is flushed in the case of Close
    //       - User selectable/created log sinks
    //       - Runtime and compile time log level switches
    match msg.contents {
        LogMessageContents::Log(contents) => {
            println!("{}", contents);
        },
        LogMessageContents::Debug(contents) => {
            println!("[DEBUG] {}", contents);
        },
        LogMessageContents::Warn(contents) => {
            println!("[WARNING] {}", contents);
        },
        LogMessageContents::Error(contents) => {
            println!("[ERROR] {}", contents);
        },
        LogMessageContents::Fatal(contents) => {
            println!("[FATAL] {}", contents);
        },
        LogMessageContents::Close => {
            println!("Closing log channel");
        },
    }
}

thread_local! {
    pub static LOG_TX_THREAD_LOCAL: RefCell<Option<Sender<LogMessage>>> = RefCell::new(None); 
}

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __msg = LogMessage::new(module_path!(), LogMessageContents::Debug(format!($($arg)*)), );
            log_send!(__tx, __msg);
        });
    }
}

#[allow(unused_macros)]
macro_rules! log {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __msg = LogMessage::new(module_path!(), LogMessageContents::Log(format!($($arg)*)), );
            log_send!(__tx, __msg);
        });
    }
}

#[allow(unused_macros)]
macro_rules! warn {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __msg = LogMessage::new(module_path!(), LogMessageContents::Warn(format!($($arg)*)), );
            log_send!(__tx, __msg);
        });
    }
}

#[allow(unused_macros)]
macro_rules! error {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __msg = LogMessage::new(module_path!(), LogMessageContents::Error(format!($($arg)*)), );
            log_send!(__tx, __msg);
        });
    }
}

#[allow(unused_macros)]
macro_rules! fatal {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __msg = LogMessage::new(module_path!(), LogMessageContents::Fatal(format!($($arg)*)), );
            log_send!(__tx, __msg);

            ::std::thread::sleep(::std::time::Duration::from_millis(100)); // hack to allow pending log messages to cleanly post
            panic!()
        });
    }
}

macro_rules! log_send {
    ($tx:expr, $msg:expr) => {
        let __tx = $tx.as_ptr();
        let __msg = $msg;
        unsafe {
            if (*__tx).is_some() {
                let __res = (*__tx).as_ref().unwrap().send(__msg);
            } else {
                __tx.replace( Some(crate::debug::get_log_channel()) );
                let __res = (*__tx).as_ref().unwrap().send(__msg);
            }
        }
    };
}





pub fn foo() {
    log!("foo");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_log() {
        for i in 0..10 {
            thread::spawn(move || {
                for j in 0..1 {
                    log!("test log {}:{}", i, j);
                    warn!("test warn {}:{}", i, j);
                    error!("test error {}:{}", i, j);
                }
            });
        }

        thread::sleep(std::time::Duration::from_millis(100));

        unsafe {
            cleanup_log_channel()
        };
    }
}
