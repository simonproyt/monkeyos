use crate::sys::{SyscallEnv, ServiceRegistry};
use crate::ipc::IpcBus;

pub type ProcessId = usize;

pub trait Process {
    fn id(&self) -> ProcessId;
    fn name(&self) -> &str;
    
    /// Called every kernel tick. 
    /// Returns true if the process is still running, false if it has exited.
    fn tick(&mut self, env: &mut SyscallEnv) -> bool;
}

pub struct ProcessManager {
    processes: Vec<Box<dyn Process>>,
    next_pid: ProcessId,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_pid: 1,
        }
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn spawn<F>(&mut self, constructor: F) -> ProcessId
    where
        F: FnOnce(ProcessId) -> Box<dyn Process>,
    {
        let pid = self.next_pid;
        self.next_pid += 1;
        let process = constructor(pid);
        self.processes.push(process);
        pid
    }

    pub fn tick_all(&mut self, ipc: &mut IpcBus, registry: &ServiceRegistry) {
        // Tick all processes, retaining only those that haven't exited
        self.processes.retain_mut(|p| {
            let mut env = SyscallEnv {
                pid: p.id(),
                ipc,
                registry,
            };
            p.tick(&mut env)
        });
    }
}
