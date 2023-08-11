use gtk::glib;
use gtk::subclass::prelude::*;
use nostr_sdk::prelude::*;

use crate::nostr::Persona;

glib::wrapper! {
    pub struct Author(ObjectSubclass<imp::Author>)
        @extends gtk::Widget,
        @implements gtk::Accessible;
}

impl Author {
    pub fn with_pubkey(pubkey: XOnlyPublicKey) -> Author {
        glib::Object::builder()
            .property("pubkey", pubkey.to_bech32().unwrap())
            .build()
    }

    pub fn set_persona(&self, persona: &Persona) {
        self.set_name(persona.name.clone());
        self.set_display_name(persona.display_name.clone());
        self.set_name_to_show(
            persona
                .display_name
                .clone()
                .or_else(|| persona.name.as_ref().map(|n| format!("@{n}")))
                .filter(|s| !s.is_empty())
                .map(|mut s| {
                    if s.len() > 60 {
                        s.truncate(60);
                        format!("{s}…")
                    } else {
                        s
                    }
                }),
        );
        self.set_nip05(persona.nip05.clone());
    }

    pub fn set_icon(&self, icon: &gtk::Image) {
        self.imp().set_icon(icon);
    }

    pub fn set_context_menu(&self, menu: Option<&gtk::gio::Menu>) {
        self.imp().set_context_menu(menu);
    }
}

mod imp {
    use std::cell::{Cell, RefCell};

    use gtk::glib;
    use gtk::glib::clone;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Author)]
    pub struct Author {
        #[property(get, set)]
        nip05_verified: Cell<bool>,
        #[property(get, set, nullable)]
        name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        display_name: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        name_to_show: RefCell<Option<String>>,
        #[property(get, set, construct_only)]
        pubkey: RefCell<String>,
        #[property(get, set, nullable)]
        nip05: RefCell<Option<String>>,
        context_menu: RefCell<Option<gtk::gio::Menu>>,
        icon: RefCell<Option<gtk::Image>>,
        author_box: RefCell<Option<gtk::Box>>,
    }

    impl Author {
        fn shortened(s: &str, chars: usize) -> String {
            let (pre, tail) = s.split_at(chars + 5);
            let pre = pre.replace("npub1", r#"<span alpha="50%">npub1</span>"#);
            let (_, post) = tail.split_at(tail.len() - chars);
            format!("{pre}…{post}")
        }

        pub fn set_icon(&self, new_icon: &gtk::Image) {
            let mut icon = self.icon.borrow_mut();
            if let Some(old_icon) = icon.take() {
                old_icon.unparent();
            };
            if let Some(ab) = &*self.author_box.borrow() {
                ab.prepend(new_icon);
                *icon = Some(new_icon.clone());
            };
        }

        pub fn set_context_menu(&self, menu: Option<&gtk::gio::Menu>) {
            *self.context_menu.borrow_mut() = menu.cloned();

            if let Some(menu) = &*self.context_menu.borrow() {
                if let Some(author_box) = &*self.author_box.borrow() {
                    let click = gtk::GestureClick::builder().button(3).build();
                    click.connect_pressed(
                        clone!(@strong author_box, @strong menu => move |_, _, x, y| {
                            let popover = gtk::PopoverMenu::builder()
                                .menu_model(&menu)
                                .has_arrow(false)
                                .pointing_to(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1))
                                .build();

                            popover.set_parent(&author_box);
                            popover.popup();
                        }),
                    );

                    author_box.add_controller(click);
                };
            };
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Author {
        const NAME: &'static str = "Author";
        type Type = super::Author;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_css_name("author");
            klass.set_accessible_role(gtk::AccessibleRole::Label);
        }
    }

    impl ObjectImpl for Author {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            relm4::view! {
                #[name = "author_box"]
                gtk::Box {
                    set_has_tooltip: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,

                    #[name = "author_name"]
                    gtk::Button {
                        add_css_class: "author-name",
                    },

                    #[name = "author_pubkey"]
                    gtk::Label {
                        add_css_class: "author-pubkey",
                        set_use_markup: true,
                    },

                    #[name = "author_nip05"]
                    gtk::Label {
                        add_css_class: "author-nip05",
                        set_yalign: 1.0,
                    },


                }
            };

            obj.bind_property("name-to-show", &author_name, "label")
                .sync_create()
                .build();
            obj.bind_property("pubkey", &author_pubkey, "label")
                .transform_to(|_, pubkey: &str| Some(Author::shortened(pubkey, 12)))
                .sync_create()
                .build();
            obj.bind_property("nip05", &author_nip05, "label")
                .transform_to(|_, nip05: Option<String>| {
                    Some(nip05.map(|n| format!("✅ {}", n.strip_prefix("_@").unwrap_or(&n))))
                })
                .sync_create()
                .build();
            obj.bind_property("name-to-show", &author_name, "visible")
                .transform_to(|_, name: Option<String>| {
                    Some(name.is_some() && name.iter().all(|s| !s.is_empty()))
                })
                .sync_create()
                .build();
            obj.bind_property("nip05-verified", &author_nip05, "visible")
                .sync_create()
                .build();
            obj.bind_property("nip05-verified", &author_pubkey, "visible")
                .invert_boolean()
                .sync_create()
                .build();

            author_box.connect_query_tooltip(
                glib::clone!(@weak obj => @default-return false, move |_, _, _, _, tooltip| {
                    let markup = format!(
                        r###"<span alpha="70%">Pubkey hex:</span> <span color="yellow">{}</span>
<span alpha="70%">Pubkey bech32:</span> <span color="#00FF00">{}</span>
<span alpha="70%">Name:</span> <b>{}</b>
<span alpha="70%">Display name:</span> <b>{}</b>
<span alpha="70%">NIP-05:</span> <span color="cyan">{}</span>
<span alpha="70%">NIP-05 verified: </span> {}"###,
                        obj.pubkey(),
                        obj.pubkey(),
                        obj.name().as_ref().unwrap_or(&"?".to_string()),
                        obj.display_name().as_ref().unwrap_or(&"?".to_string()),
                        obj.nip05().as_ref().unwrap_or(&"?".to_string()),
                        obj.nip05_verified()
                    );

                    tooltip.set_markup(Some(&markup));
                    true
                }),
            );

            author_box.set_parent(&*obj);
            *self.author_box.borrow_mut() = Some(author_box);
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn dispose(&self) {
            if let Some(b) = self.author_box.borrow_mut().take() {
                b.unparent();
            }
        }
    }

    impl WidgetImpl for Author {}
}
