use binaryninja::custom_binary_view::register_view_type;
use log::{info};

mod view;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn CorePluginInit() -> bool {
    info!("The logger has been initialized!");

    register_view_type("iBoot", "iBoot", view::iBootViewType::new);

    true
}
