
use std::{cell::{Cell, RefCell}, sync::{Mutex, MutexGuard, Once, mpsc::{ channel, Sender, Receiver }}, thread::{self, JoinHandle, ThreadId}, time::{Duration, SystemTime, UNIX_EPOCH}};

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

impl LogMessageContents {
    fn type_string(&self) -> &str {
        match self {
            Self::Log(_) => "log",
            Self::Warn(_) => "warning",
            Self::Error(_) => "error",
            Self::Debug(_) => "debug",
            Self::Fatal(_) => "fatal",
            Self::OpenContext => "open_context",
            Self::CloseContext => "close_context",
            Self::HorizontalLine => "horizontal_line",
            Self::Close => "close",
        }
    }
}

#[derive(Debug)]
pub struct LogMessage {
    time: SystemTime,
    thread: ThreadId,
    module: &'static str,
    context: Option<(usize, &'static str)>,
    contents: LogMessageContents,
}

impl LogMessage {
    pub fn new(context: Option<(usize, &'static str)>, module: &'static str, contents: LogMessageContents) -> Self {
        LogMessage {
            time: SystemTime::now(),
            thread: thread::current().id(),
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

trait LogSink {
    fn handle_message(&self, message: LogMessage);
}

/// The default log sink
///
/// Stateless. Simply prints log messages to standard out as it receives them
struct DefaultLogSink {}

impl DefaultLogSink {
    fn new() -> Self {
        Self {}
    }
}

impl LogSink for DefaultLogSink {
    fn handle_message(&self, message: LogMessage) {
        let message_context = message.context;
        let time_seconds = message.time.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let time_subsec = message.time.duration_since(UNIX_EPOCH).unwrap_or_default().subsec_micros();
        
        let context_string = match message_context {
            Some((_nesting, context)) => {
                format!("{}:{}", message.contents.type_string(), context.split_whitespace().collect::<String>())
            },
            None => {
                format!("{}", message.contents.type_string())
            }
        };
        
        //println!("\x1b[1;31mbold red text\x1b[0m");
        //println!("\x1b[1;93mbold yellow text\x1b[0m");
        
        let time_string = format!("{}.{}", time_seconds, time_subsec);

        //let contents = "test message";
        //println!("\n{time} - {cont}", time=time_string, cont=contents);
        //println!("{time} [\x1b[0;35m{ctx}\x1b[0m] - {cont}", ctx="debug:testmodule", time=time_string, cont=contents);
        //println!("{time} [\x1b[0;33m{ctx}\x1b[0m] - {cont}", ctx="warning:testmodule", time=time_string, cont=contents);
        //println!("{time} [\x1b[0;31m{ctx}\x1b[0m] - {cont}", ctx="error:testmodule", time=time_string, cont=contents);
        //println!("\x1b[0;41m{time} [{ctx}] - {cont}\x1b[0m", ctx="fatal:testmodule".to_uppercase(), time=time_string, cont=contents.to_uppercase());
        //println!("closing log channel");

        match &message.contents {
            LogMessageContents::Log(contents) => {
                println!("{time} - {cont}", time=time_string, cont=contents);
            },
            LogMessageContents::Debug(contents) => {
                println!("{time} [\x1b[0;35m{ctx}\x1b[0m] - {cont}", ctx=context_string, time=time_string, cont=contents);
            },
            LogMessageContents::Warn(contents) => {
                println!("{time} [\x1b[0;33m{ctx}\x1b[0m] - {cont}", ctx=context_string, time=time_string, cont=contents);
            },
            LogMessageContents::Error(contents) => {
                println!("{time} [\x1b[0;31m{ctx}\x1b[0m] - {cont}", ctx=context_string, time=time_string, cont=contents);
            }
            LogMessageContents::Fatal(contents) => {
                println!("\x1b[1;41m{time} [{ctx}] - {cont}\x1b[0m", ctx=context_string.to_uppercase(), time=time_string, cont=contents.to_uppercase());
            },
            LogMessageContents::Close => {
                // do nothing
            },
            LogMessageContents::HorizontalLine => {
                // do nothing
            },
            LogMessageContents::OpenContext => {
                // do nothing
            },
            LogMessageContents::CloseContext => {
                // do nothing
            }
        }
    }
}

fn set_log_sink(sink: Box<dyn LogSink>) -> Option<Box<dyn LogSink>> {
    unsafe {
        match LOG_SINK.get_mut() {
            Some(mutex) => {
                match mutex.lock() {
                    Ok(mut old_sink) => {
                        return Some(std::mem::replace(&mut *old_sink, sink));
                    },
                    Err(e) => {
                        todo!("handle log sink mutex poisoning, {:?}", e);
                    }
                }
            },
            None => {
                LOG_SINK.set(Some(Mutex::new(sink)));
            }
        }
        return None
    }
}

static mut LOG_INIT_ONCE: Once = Once::new();
static mut LOG_SENDER_MUTEX: LogTx = LogTx::new();
static mut LOG_RECEIVER_MUTEX: LogRx = LogRx::new();
static mut LOG_RECEIVER_JOIN_HANDLE: Cell<Option<JoinHandle<RecvGuardTunnel>>> = Cell::new(None);
static mut LOG_SINK: Cell<Option<Mutex<Box<dyn LogSink>>>> = Cell::new(None);

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

/// Reciever thread function. There should only ever be one of these running
fn log_reciever_fn(guard_tunnel: RecvGuardTunnel) -> RecvGuardTunnel {
    let receiver_guard = guard_tunnel.0;

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
            Err(_e) => {
                // TODO: do something with the RecvError
                return RecvGuardTunnel(receiver_guard);
            }
        }
            
        if close {
            return RecvGuardTunnel(receiver_guard);
        }
    }
}

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

        // Setup the log sink
        set_log_sink(Box::new(DefaultLogSink::new()));

        // Spawn the reciever thread
        let handle = thread::spawn(move || log_reciever_fn(guard_tunnel));

        LOG_RECEIVER_JOIN_HANDLE.set(Some(handle));
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
        time: SystemTime::now(),
        thread: thread::current().id(),
        module: module_path!(),
        context: None,
        contents: LogMessageContents::Close,
    });

    match LOG_RECEIVER_JOIN_HANDLE.take() {
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
    // TODO: - Buffer messages as they come in to reduce mutex locks
    // TODO: - Sink that periodically dumps contextual data into the output
    //       - Ensure all data is flushed in the case of Close
    //       - Runtime and compile time log level switches

    // TODO: - When printing messages with context from multiple threads try to buffer a few messages
    //         and then print one threads messages all at once, or as many as you can, with the full
    //         context and indent tree preceding them each time

    // TODO: - better/more efficient mechanism than a mutex for mainting the log sink/swapping it
    
    match unsafe { &*LOG_SINK.as_ptr() } {
        Some(sink) => {
            sink.lock().unwrap().handle_message(msg);
        },
        None => {
            set_log_sink(Box::new(DefaultLogSink::new()));
            sink(msg);
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
                                __tx.context = Some((__context.0 + 1, $name));
                            },
                            None => {
                                __tx.context = Some((0usize, $name));
                            },
                        };
                    },
                    None => {
                        __tx.replace(Some(LogDevice::with_context(Some((0usize, $name)))));
                    },
                };
            }

            $(
                $body
            )*

            {
                __tx.replace(Some(LogDevice::from_original(__tx.take(), __previous_context)));
            }
        })
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_log() {
        log!("plain log message");
        debug!("debug log message");
        warn!("warning log message");
        error!("error log message");

        thread::sleep(std::time::Duration::from_millis(100));

        unsafe {
            cleanup_log_channel()
        };
    }

    #[test]
    #[should_panic]
    fn test_fatal_log() {
        fatal!("fatal log message"); // this should panic
    }
}
