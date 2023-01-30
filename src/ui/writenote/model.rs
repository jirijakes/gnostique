#[derive(Debug)]
pub struct WriteNote {
    pub visible: bool,
    pub buffer: gtk::TextBuffer,
}

#[derive(Debug)]
pub enum WriteNoteInput {
    Hide,
    Cancel,
    Show,
    Send,
}

#[derive(Debug)]
pub enum WriteNoteResult {
    Cancel,
    Send(String),
}
