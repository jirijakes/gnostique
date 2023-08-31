use gtk::prelude::*;
use relm4::component::*;
use relm4::*;

use super::model::*;
use super::msg::*;
use crate::ui::main::Main;
use crate::ui::settings::Settings;
use crate::ui::unlock::{Unlock, UnlockResult};

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name(window)]
        gtk::ApplicationWindow {

            #[name(stack)]
            gtk::Stack {
                set_hexpand: true,
                set_vexpand: true,

                #[local_ref]
                unlock -> gtk::Stack { }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let unlock = Unlock::builder()
            .launch(())
            .forward(sender.input_sender(), |result| match result {
                UnlockResult::Quit => AppInput::Quit,
                UnlockResult::Unlocked(gn) => AppInput::Unlocked(gn),
            });

        let settings = Settings::builder().launch(()).detach();

        let model = App {
            main: None,
            unlock,
            settings,
        };

        let unlock = model.unlock.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppInput::Quit => relm4::main_application().quit(),
            AppInput::Unlocked(gn) => {
                let main = Main::builder().launch(gn).detach();
                widgets.stack.add_named(main.widget(), Some("main"));
                self.main = Some(main);
                widgets.stack.set_visible_child_name("main");
            }
            // AppInput::ShowSettings => self
                // .settings
                // .emit(SettingsInput::Show(self.gnostique.identities())),

        }

        self.update_view(widgets, sender);
    }
}
