use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use std::collections::HashMap;

pub struct VfsService {
    pid: ProcessId,
    // In-memory filesystem mock
    files: HashMap<String, String>,
}

impl VfsService {
    pub fn new(pid: ProcessId) -> Self {
        let mut files = HashMap::new();
        files.insert("/etc/hostname".to_string(), "monkeyos".to_string());
        Self { pid, files }
    }
}

impl Process for VfsService {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "vfs_server" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        while let Some(msg) = env.recv_msg() {
            // we dropped sender info for simplicity but let's assume we log it
            crate::log("[VFS] Received request");
        }
        true
    }
}
