pub struct CtrlMessage {
    pub command: Ctrl,
}
pub enum Ctrl {
    RequestFrame,
    Stop,
}

pub struct ScreenMsg{
    pub screen_type: ScreenType,
    pub data: Vec<u8>,
}
pub enum ScreenType {
    TextMode,
    FrameBuffer,
}