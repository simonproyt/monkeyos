use crate::process::{Process, ProcessId};
use crate::sys::SyscallEnv;
use crate::ipc::MessagePayload;

pub struct InputServer {
    pid: ProcessId,
}

impl InputServer {
    pub fn new(pid: ProcessId) -> Self {
        Self { pid }
    }
}

impl Process for InputServer {
    fn id(&self) -> ProcessId { self.pid }
    fn name(&self) -> &str { "input_server" }

    fn tick(&mut self, env: &mut SyscallEnv) -> bool {
        // The InputServer receives messages from the JS boundary (via Kernel push_input_event)
        // and forwards them to the Window Manager.
        while let Some(msg) = env.recv_msg() {
            if let Some(wm_pid) = env.lookup_service("wm") {
                match msg.payload {
                    MessagePayload::MouseMove { x, y } => {
                        env.send_msg(wm_pid, MessagePayload::MouseMove { x, y });
                    }
                    MessagePayload::MouseButton { down } => {
                        env.send_msg(wm_pid, MessagePayload::MouseButton { down });
                    }
                    MessagePayload::KeyPress { key_code } => {
                        // Forward keystrokes directly to the terminal process
                        if let Some(term_pid) = env.lookup_service("terminal") {
                            env.send_msg(term_pid, MessagePayload::KeyPress { key_code });
                        }
                    }
                    _ => {}
                }
            }
        }
        true
    }
}
