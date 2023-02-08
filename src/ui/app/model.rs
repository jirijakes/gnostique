use relm4::component::AsyncController;

use crate::ui::main::Main;

#[derive(Default)]
pub struct App {
    pub(super) main: Option<AsyncController<Main>>,
}
