use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    io::Write,
    time::{Duration, Instant},
};
use tracing_subscriber::EnvFilter;

const DEFAULT_FILTER: &str = "info,shrimply=debug";
const TIMING_LOG_INTERVAL: Duration = Duration::from_secs(1);

thread_local! {
    static TIMINGS: RefCell<HashMap<&'static str, (u64, Duration, Duration)>> = RefCell::new(HashMap::new());
    static COUNTS: RefCell<BTreeMap<&'static str, u64>> = const { RefCell::new(BTreeMap::new()) };
    static WINDOW: RefCell<Instant> = RefCell::new(Instant::now());
}

/// Plain periodic log summaries; independent of the performance inspector.
pub fn timing(stage: &'static str) -> Timing {
    Timing {
        stage,
        started: Instant::now(),
    }
}

pub struct Timing {
    stage: &'static str,
    started: Instant,
}

impl Drop for Timing {
    fn drop(&mut self) {
        record_timing(self.stage, self.started.elapsed());
    }
}

pub fn record_timing(stage: &'static str, elapsed: Duration) {
    TIMINGS.with(|timings| {
        let mut timings = timings.borrow_mut();
        let (calls, total, maximum) = timings.entry(stage).or_default();
        *calls += 1;
        *total += elapsed;
        *maximum = (*maximum).max(elapsed);
    });
}

/// Count causes, including multiple causes for one redraw, without logging per frame.
pub fn count(event: &'static str) {
    COUNTS.with(|counts| *counts.borrow_mut().entry(event).or_default() += 1);
}

/// Called at the start of the UI callback: all completed stages share one window,
/// including stages that stopped running. Sparse draws cannot retain old samples.
pub fn flush_timings() {
    let window = WINDOW.with(|since| {
        let mut since = since.borrow_mut();
        let now = Instant::now();
        let elapsed = now.duration_since(*since);
        if elapsed < TIMING_LOG_INTERVAL {
            return None;
        }
        *since = now;
        Some(elapsed)
    });
    let Some(window) = window else {
        return;
    };
    let window_us = window.as_micros();
    TIMINGS.with(|timings| {
        for (stage, samples) in timings.borrow_mut().iter_mut() {
            let (calls, total, maximum) = std::mem::take(samples);
            let total_us = total.as_micros();
            tracing::info!(
                stage,
                window_us,
                calls,
                total_us,
                average_us = total_us.checked_div(u128::from(calls)).unwrap_or(0),
                max_us = maximum.as_micros(),
                "UI lifecycle timing"
            );
        }
    });
    COUNTS.with(|counts| {
        let mut counts = counts.borrow_mut();
        tracing::info!(window_us, counts = ?*counts, "UI lifecycle counts");
        for count in counts.values_mut() {
            *count = 0;
        }
    });
}

#[derive(Clone, Copy, Debug)]
pub struct ResilientWriter<W> {
    inner: W,
}

impl<W> ResilientWriter<W> {
    pub const fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for ResilientWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.inner.write(buf) {
            Ok(n) => Ok(n),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::BrokenPipe =>
            {
                // Diagnostic logging is best-effort. If the output stream is congested
                // or unavailable, safely drop this slice without blocking or failing write_all.
                Ok(buf.len())
            }
            Err(error) => Err(error),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.inner.flush() {
            Ok(()) => Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::BrokenPipe =>
            {
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}

fn make_stderr() -> ResilientWriter<std::io::Stderr> {
    ResilientWriter::new(std::io::stderr())
}

/// Installs the process-wide diagnostics subscriber.
///
/// Application code emits `tracing` spans and events. Logs emitted by dependencies through the
/// `log` facade are forwarded by `tracing-subscriber` into the same output.
pub fn init() {
    let filter = EnvFilter::new(std::env::var("RUST_LOG").map_or_else(
        |_| DEFAULT_FILTER.to_string(),
        |directives| format!("{DEFAULT_FILTER},{directives}"),
    ));
    tracing_subscriber::fmt()
        .with_writer(make_stderr)
        .with_env_filter(filter)
        .with_thread_ids(true)
        .with_thread_names(true)
        .log_internal_errors(false)
        .try_init()
        .expect("diagnostics subscriber should only be installed once");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Error, ErrorKind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockWriter {
        error_to_return: Option<ErrorKind>,
        write_count: AtomicUsize,
        bytes_written: std::sync::Mutex<Vec<u8>>,
    }

    impl MockWriter {
        fn with_error(kind: ErrorKind) -> Self {
            Self {
                error_to_return: Some(kind),
                write_count: AtomicUsize::new(0),
                bytes_written: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn healthy() -> Self {
            Self {
                error_to_return: None,
                write_count: AtomicUsize::new(0),
                bytes_written: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_count.fetch_add(1, Ordering::SeqCst);
            if let Some(kind) = self.error_to_return {
                return Err(Error::new(kind, "mock error"));
            }
            self.bytes_written.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if let Some(kind) = self.error_to_return {
                return Err(Error::new(kind, "mock error"));
            }
            Ok(())
        }
    }

    #[test]
    fn absorbs_would_block_without_error_or_infinite_loop() {
        let mock = MockWriter::with_error(ErrorKind::WouldBlock);
        let mut resilient = ResilientWriter::new(mock);
        let result = resilient.write_all(b"test log event");
        assert!(
            result.is_ok(),
            "write_all must succeed when stream would block"
        );
        assert_eq!(resilient.inner.write_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn absorbs_broken_pipe_without_error() {
        let mock = MockWriter::with_error(ErrorKind::BrokenPipe);
        let mut resilient = ResilientWriter::new(mock);
        let result = resilient.write_all(b"test log event");
        assert!(
            result.is_ok(),
            "write_all must succeed when stream has broken pipe"
        );
    }

    #[test]
    fn writes_normally_when_stream_is_healthy() {
        let mock = MockWriter::healthy();
        let mut resilient = ResilientWriter::new(mock);
        let result = resilient.write_all(b"test log event");
        assert!(result.is_ok());
        assert_eq!(
            resilient.inner.bytes_written.lock().unwrap().as_slice(),
            b"test log event"
        );
    }

    #[test]
    fn flush_absorbs_would_block_and_broken_pipe() {
        let mut resilient_wb = ResilientWriter::new(MockWriter::with_error(ErrorKind::WouldBlock));
        assert!(resilient_wb.flush().is_ok());

        let mut resilient_bp = ResilientWriter::new(MockWriter::with_error(ErrorKind::BrokenPipe));
        assert!(resilient_bp.flush().is_ok());
    }

    #[test]
    fn propagates_unrelated_serious_io_errors() {
        let mock = MockWriter::with_error(ErrorKind::PermissionDenied);
        let mut resilient = ResilientWriter::new(mock);
        let result = resilient.write_all(b"test log event");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::PermissionDenied);
    }
}
