// src/wasi.js
// Polyfill for WASI imports emitted by wasm32-wasip1 target.

export function fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) {
    if (window.__WASI_PROXY && window.__WASI_PROXY.fd_write) {
        return window.__WASI_PROXY.fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr);
    }
    console.warn('Unhandled WASI fd_write');
    return 0; // Success
}

export function fd_read(fd, iovs_ptr, iovs_len, nread_ptr) {
    if (window.__WASI_PROXY && window.__WASI_PROXY.fd_read) {
        return window.__WASI_PROXY.fd_read(fd, iovs_ptr, iovs_len, nread_ptr);
    }
    console.warn('Unhandled WASI fd_read');
    return 0;
}

export function environ_get(environ, environ_buf) {
    return 0;
}

export function environ_sizes_get(environ_count_ptr, environ_buf_size_ptr) {
    return 0;
}

export function proc_exit(rval) {
    console.log("WASI proc_exit", rval);
}

export function path_open() { 
    console.warn("WASI path_open called");
    return 52; // ENOSYS
}

export function fd_close() { return 0; }
export function fd_seek() { return 0; }
export function fd_fdstat_get() { return 0; }
export function fd_prestat_get() { return 8; /* EBADF */ }
export function fd_prestat_dir_name() { return 28; /* EINVAL */ }
