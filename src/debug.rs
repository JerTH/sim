
use std::{cell::{Cell, RefCell}, sync::{Mutex, MutexGuard, Once, mpsc::{ channel, Sender, Receiver }}, thread::{self, JoinHandle, ThreadId}, time::Instant};

// TODO
//   Implement debug instrumentation and built in profiling
//
//   Logging:
//     add compile time switches
//     add log contexts,
//     add buffering of context info,
//     add context indention levels,
//     add detail switches,
//     add internal filter,
//     properly handle program termination and partial log dumps,
//     add file support


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
    OpenContext,
    CloseContext,
    HorizontalLine,
    Close,
}

#[derive(Debug)]
pub struct LogMessage {
    time: Instant,
    thread: ThreadId,
    module: &'static str,
    context: Option<(usize, &'static str)>,
    contents: LogMessageContents,
}

impl LogMessage {
    pub fn new(context: Option<(usize, &'static str)>, module: &'static str, contents: LogMessageContents) -> Self {
        LogMessage {
            time: ::std::time::Instant::now(),
            thread: ::std::thread::current().id(),
            module: module,
            context: context,
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
        // can be passed directly to the log sink thread without unlocking it,
        // spawn the sink thread and hand over the guard. The sink thread can then
        // process any pending log messages that were sent before it was created
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
        context: None,
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
    // TODO: make this reference a lazy static sink struct that can be stateful
    // TODO: - Write messages to some structure, periodically dump data into the output
    //       - Ensure all data is flushed in the case of Close
    //       - User selectable/created log sinks
    //       - Runtime and compile time log level switches

    // when printing messages with context from multiple threads try to buffer a few messages
    // and then print one threads messages all at once, or as many as you can, with the full
    // context and indent tree preceding them each time

    let tab_spacing = 4;
    let (indent, context) = msg.context.unwrap_or_default();
    let indent = indent * tab_spacing;
    match &msg.contents {
        LogMessageContents::Log(contents) => {
            println!("{}", contents);
        },
        LogMessageContents::Debug(contents) => {
            println!("[DEBUG]{:ind$}({ctx}) {con}", "", ctx=context, con=contents, ind=indent);
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
        LogMessageContents::HorizontalLine => {
            println!("{:ind$}", "-", ind=40);
        },
        LogMessageContents::OpenContext => {

        },
        LogMessageContents::CloseContext => {
            
        }
    }
}

pub struct LogDevice {
    pub context: Option<(usize, &'static str)>,
    pub tx: Sender<LogMessage>,
}

impl LogDevice {
    pub fn with_context(context: Option<(usize, &'static str)>) -> Self {
        let mut device = Self::default();
        device.context = context;
        device
    }

    pub fn from_original(original: Option<Self>, context: Option<(usize, &'static str)>) -> Self {
        match original {
            Some(device) => {
                return LogDevice {
                    tx: device.tx,
                    context: context,
                };
            },
            None => {
                return LogDevice::with_context(context);
            }
        }
    }
}

impl Default for LogDevice {
    fn default() -> Self {
        LogDevice {
            context: None,
            tx: unsafe { crate::debug::get_log_channel() },
        }
    }
}

thread_local! {
    pub static LOG_TX_THREAD_LOCAL: RefCell<Option<LogDevice>> = RefCell::new(None); 
}

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(any(debug_assertions, feature = "debug_log"))]
        {
            LOG_TX_THREAD_LOCAL.with(|__tx| {
                let __contents = LogMessageContents::Debug(format!($($arg)*));
                log_send!(__tx, __contents);
            });
        }
    }
}

#[allow(unused_macros)]
macro_rules! log {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __contents = LogMessageContents::Log(format!($($arg)*));
            log_send!(__tx, __contents);
        });
    }
}

#[allow(unused_macros)]
macro_rules! warn {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __contents = LogMessageContents::Warn(format!($($arg)*));
            log_send!(__tx, __contents);
        });
    }
}

#[allow(unused_macros)]
macro_rules! error {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __contents = LogMessageContents::Error(format!($($arg)*));
            log_send!(__tx, __contents);
        });
    }
}

#[allow(unused_macros)]
macro_rules! fatal {
    ($($arg:tt)*) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let __contents = LogMessageContents::Fatal(format!($($arg)*));
            log_send!(__tx, __contents);

            // hack to allow pending log messages to (hopefully) post before we kill the process
            ::std::thread::sleep(::std::time::Duration::from_millis(100)); 
            panic!()
        });
    }
}

macro_rules! log_send {
    ($tx:expr, $contents:expr) => {
        let __contents = $contents;

        #[allow(unused_unsafe)] // not actually unused, but we get a warning otherwise, probably a compiler bug
        unsafe {
            match &mut *$tx.as_ptr() {
                Some(__tx) => {
                    let __msg = LogMessage::new(__tx.context, module_path!(), __contents);
                    let __res = __tx.tx.send(__msg);
                },
                None => {
                    $tx.replace(Some(LogDevice::default()));
                    match &mut *$tx.as_ptr() {
                        Some(__tx) => {
                            let __msg = LogMessage::new(__tx.context, module_path!(), __contents);
                            let __res = __tx.tx.send(__msg);
                        },
                        None => {
                            unreachable!()
                        }
                    }
                }
            }
        }
    };
}

#[allow(unused_macros)]
macro_rules! log_context {
    (($name:expr) {$($body:tt)*}) => {
        LOG_TX_THREAD_LOCAL.with(|__tx| {
            let mut __previous_context: Option<(usize, &'static str)> = None;
            
            unsafe {
                match &mut *__tx.as_ptr() {
                    Some(__tx) => {
                        __previous_context = __tx.context;
                        match __tx.context {
                            Some(mut __context) => {
                                //println!("setting context to {:?}, Some, Some", $name);
                                __tx.context = Some((__context.0 + 1, $name));
                                //println!("context: {:?}", __context);
                            },
                            None => {
                                //println!("setting context to {:?}, Some, None", $name);
                                __tx.context = Some((0usize, $name));
                            },
                        };
                        //println!("context: {:?}", __tx.context);
                    },
                    None => {
                        //println!("setting context to {:?}, None", $name);
                        __tx.replace(Some(LogDevice::with_context(Some((0usize, $name)))));
                    },
                };
            }

            $(
                $body
            )*

            {
                //let __temp = __tx.take();
                //println!("setting context to {:?}", __previous_context);
                __tx.replace(Some(LogDevice::from_original(__tx.take(), __previous_context)));
            }
        })
    };
}



pub fn foo() {
    log!("foo");
    
    let mut a = 0;

    log_context!(("foo") {
        a += 2;
    })
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
