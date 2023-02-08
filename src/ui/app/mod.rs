//! `App` is the outermost widget of Gnostique, the omnicontaining
//! [`ApplicationWindow`](gtk::ApplicationWindow). Everything starts
//! here and with the end of `App`, the end of Gnostique arrives.
//!
//! `App` does not hold state, it mainly asks user to provide access to
//! secrets (either by password or by providing them directly) and then
//! redirects all life into [`Main`](crate::ui::main).

mod model;
mod view;
mod msg;

pub use model::App;
