use binaryninja::custombinaryview::register_view_type;
use log::{debug, LevelFilter};

mod view;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn CorePluginInit() -> bool {
    binaryninja::logger::init(LevelFilter::Trace).expect("failed to initialize logging");

    register_view_type("iBoot", "iBoot", view::iBootViewType::new);

    true
}
