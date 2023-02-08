use relm4::component::AsyncController;
use relm4::Controller;

use crate::ui::main::Main;
use crate::ui::unlock::Unlock;

pub struct App {
    pub(super) main: Option<AsyncController<Main>>,
    pub(super) unlock: Controller<Unlock>,
}
