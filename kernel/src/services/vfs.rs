use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;

pub struct VfsService {
    pid: ProcessId,
}

impl VfsService {
    pub fn new(pid: ProcessId) -> Self {
        Self { pid }
    }
}

impl Process for VfsService {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "vfs_server" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        while let Some(_msg) = env.recv_msg() {
            // we dropped sender info for simplicity but let's assume we log it
            crate::log("[VFS] Received request");
        }
        true
    }
}
