use crate::ipc::{IpcBus, Message, MessagePayload};
use crate::process::ProcessId;
use std::collections::HashMap;

// A simple registry to look up service PIDs by name
pub struct ServiceRegistry {
    services: HashMap<String, ProcessId>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self { services: HashMap::new() }
    }

    pub fn register(&mut self, name: &str, pid: ProcessId) {
        self.services.insert(name.to_string(), pid);
    }

    pub fn lookup(&self, name: &str) -> Option<ProcessId> {
        self.services.get(name).copied()
    }
}

pub struct SyscallEnv<'a> {
    pub pid: ProcessId,
    pub ipc: &'a mut IpcBus,
    pub registry: &'a ServiceRegistry,
}

impl<'a> SyscallEnv<'a> {
    pub fn send_msg(&mut self, target: ProcessId, payload: MessagePayload) {
        self.ipc.send(Message {
            sender: self.pid,
            receiver: target,
            payload,
        });
    }

    pub fn recv_msg(&mut self) -> Option<Message> {
        // Return the full Message so the receiver knows the sender PID
        self.ipc.receive(self.pid)
    }

    pub fn lookup_service(&self, name: &str) -> Option<ProcessId> {
        self.registry.lookup(name)
    }
}
