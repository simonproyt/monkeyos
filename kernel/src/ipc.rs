use std::collections::{HashMap, VecDeque};
use crate::process::ProcessId;

#[derive(Clone, Debug)]
pub enum MessagePayload {
    Ping,
    Pong,
    DrawRect { x: i32, y: i32, w: i32, h: i32 },
    ClearScreen,
    MouseMove { x: i32, y: i32 },
    MouseButton { down: bool },
    KeyPress { key_code: u32 },
    CreateHtmlOverlay { id: u32, x: i32, y: i32, w: i32, h: i32 },
    UpdateHtmlOverlayPos { id: u32, x: i32, y: i32 },
    AppendHtmlOverlayText { id: u32, text: String },
    UpdateHtmlOverlayInputLine { id: u32, prompt: String, input: String, cursor_pos: u32 },
    ClearHtmlOverlayText { id: u32 },
    // Other syscalls / service messages will go here
}

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: ProcessId,
    pub receiver: ProcessId,
    pub payload: MessagePayload,
}

pub struct IpcBus {
    mailboxes: HashMap<ProcessId, VecDeque<Message>>,
}

impl IpcBus {
    pub fn new() -> Self {
        Self {
            mailboxes: HashMap::new(),
        }
    }

    pub fn send(&mut self, msg: Message) {
        self.mailboxes
            .entry(msg.receiver)
            .or_insert_with(VecDeque::new)
            .push_back(msg);
    }

    pub fn receive(&mut self, pid: ProcessId) -> Option<Message> {
        self.mailboxes.get_mut(&pid).and_then(|queue| queue.pop_front())
    }
}
