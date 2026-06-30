use crate::sys::SyscallEnv;

pub struct FileHandle {
    // fd: usize
}

pub fn open(env: &mut SyscallEnv, path: &str) -> Option<FileHandle> {
    if let Some(_vfs_pid) = env.lookup_service("vfs") {
        crate::log(&std::format!("[API] Opening file: {}", path));
        Some(FileHandle {})
    } else {
        None
    }
}
