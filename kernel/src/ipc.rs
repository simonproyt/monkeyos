use std::collections::{HashMap, VecDeque};
use crate::process::ProcessId;

#[derive(Clone, Debug)]
pub enum MessagePayload {
    CreateWindow { id: u32, x: i32, y: i32, w: i32, h: i32, title: String, owner: ProcessId },
    DrawRect { x: i32, y: i32, w: i32, h: i32, r: f32, g: f32, b: f32, a: f32, radius: f32, shadow_blur: f32 },
    ClearScreen,
    ScreenSizeChanged { w: i32, h: i32 },
    MouseMove { x: i32, y: i32 },
    MouseButton { down: bool },
    KeyPress { key_code: u32 },
    UpdateTerminalBuffer { id: u32, lines: Vec<String>, is_pager: bool, pager_title: Option<String>, scroll_y: usize },
    WasiPrintChar { id: u32, c: u8 },
    WindowClosed { id: u32 },
    DrawText { x: i32, y: i32, text: String, font_size: f32, r: f32, g: f32, b: f32, a: f32 },
    DrawCenteredText { x: i32, y: i32, text: String, font_size: f32, r: f32, g: f32, b: f32, a: f32 },
    DrawGuiApp { id: u32, x: i32, y: i32, w: i32, h: i32 },
    LoadImage { id: u32, url: String },
    DrawImage { id: u32, x: i32, y: i32, w: i32, h: i32 },
    ClearText,
    SpawnTerminal,
    SpawnProcess { bin: String },
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
            .or_default()
            .push_back(msg);
    }

    pub fn receive(&mut self, pid: ProcessId) -> Option<Message> {
        self.mailboxes.get_mut(&pid).and_then(|queue| queue.pop_front())
    }
}

impl Default for IpcBus {
    fn default() -> Self {
        Self::new()
    }
}
