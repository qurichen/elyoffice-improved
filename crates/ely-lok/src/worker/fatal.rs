//! Fatal signals print the native stack first.

use std::ffi::{c_int, c_void};

unsafe extern "C" {
    fn backtrace(buffer: *mut *mut c_void, size: c_int) -> c_int;
    fn backtrace_symbols_fd(buffer: *const *mut c_void, size: c_int, fd: c_int);
}

extern "C" fn report(signal: c_int) {
    let mut frames = [std::ptr::null_mut(); 64];
    // As a log record, so the app's relay keeps it.
    let note = b"[fatal ERROR ely_lok::worker::fatal] native stack follows\n";
    unsafe {
        libc::write(2, note.as_ptr().cast(), note.len());
        let count = backtrace(frames.as_mut_ptr(), frames.len() as c_int);
        backtrace_symbols_fd(frames.as_ptr(), count, 2);
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

/// Prints a stack for crash signals.
pub(super) fn watch() {
    for signal in [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGABRT] {
        unsafe { libc::signal(signal, report as *const () as libc::sighandler_t) };
    }
}
