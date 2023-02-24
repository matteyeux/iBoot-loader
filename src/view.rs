use binaryninja::symbol::Symbol;
use binaryninja::symbol::SymbolType;
use std::ops::{Deref};
use std::sync::Arc;
use binaryninja::architecture::CoreArchitecture;
use binaryninja::architecture::ArchitectureExt;
use binaryninja::section::{Section, Semantics};
use binaryninja::segment::Segment;
use log::{debug, error, info, warn};

use binaryninja::binaryview::{BinaryView, BinaryViewBase, BinaryViewExt};
use binaryninja::custombinaryview::{
    BinaryViewType, BinaryViewTypeBase, CustomBinaryView, CustomBinaryViewType, CustomView,
    CustomViewBuilder,
};
use binaryninja::databuffer::DataBuffer;
use binaryninja::Endianness;

type BinaryViewResult<R> = binaryninja::binaryview::Result<R>;

/// A wrapper around a `binaryninja::databuffer::DataBuffer`, from which a `[u8]` buffer can be obtained
/// to pass to `minidump::Minidump::read`.
///
/// This code is taken from [`dwarfdump`](https://github.com/Vector35/binaryninja-api/blob/9d8bc846bd213407fb1a7a19af2a96f17501ac3b/rust/examples/dwarfdump/src/lib.rs#L81)
/// in the Rust API examples.
#[derive(Clone)]
pub struct DataBufferWrapper {
    inner: Arc<DataBuffer>,
}

impl DataBufferWrapper {
    pub fn new(buf: DataBuffer) -> Self {
        DataBufferWrapper {
            inner: Arc::new(buf),
        }
    }
}

impl Deref for DataBufferWrapper {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.inner.get_data()
    }
}

/// The _Minidump_ binary view type, which the Rust plugin registers with the Binary Ninja core
/// (via `binaryninja::custombinaryview::register_view_type`) as a possible binary view
/// that can be applied to opened binaries.
///
/// If this view type is valid for an opened binary (determined by `is_valid_for`),
/// the Binary Ninja core then uses this view type to create an actual instance of the _Minidump_
/// binary view (via `create_custom_view`).
#[allow(non_camel_case_types)]
pub struct iBootViewType {
    view_type: BinaryViewType,
}

impl iBootViewType {
    pub fn new(view_type: BinaryViewType) -> Self {
        iBootViewType { view_type }
    }
}

impl AsRef<BinaryViewType> for iBootViewType {
    fn as_ref(&self) -> &BinaryViewType {
        &self.view_type
    }
}

impl BinaryViewTypeBase for iBootViewType {
    fn is_deprecated(&self) -> bool {
        false
    }

    fn is_valid_for(&self, _data: &BinaryView) -> bool {
        //let mut magic_number = Vec::<u8>::new();
        //data.read_into_vec(&mut magic_number, 0, 4);

        //magic_number == b"MDMP"
        true
    }
}

impl CustomBinaryViewType for iBootViewType {
    fn create_custom_view<'builder>(
        &self,
        data: &BinaryView,
        builder: CustomViewBuilder<'builder, Self>,
    ) -> BinaryViewResult<CustomView<'builder>> {
        info!("Creating iBootView from registered iBootViewType");

        let binary_view = builder.create::<iBootView>(data, ());
        binary_view
    }
}


/// An instance of the actual _Minidump_ custom binary view.
/// This contains the main logic to load the memory segments inside a minidump file into the binary view.
#[allow(non_camel_case_types)]
pub struct iBootView {
    /// The handle to the "real" BinaryView object, in the Binary Ninja core.
    inner: binaryninja::rc::Ref<BinaryView>,
}

impl iBootView {
    fn new(view: &BinaryView) -> Self {
        iBootView {
            inner: view.to_owned(),
        }
    }

    fn init(&self) -> BinaryViewResult<()> {
        let parent_view = self.parent_view()?;
        let _read_buffer = parent_view.read_buffer(0, parent_view.len())?;
        let _read_buffer = DataBufferWrapper::new(_read_buffer);

        let arch = CoreArchitecture::by_name("aarch64").ok_or(())?;
        let plat = arch.standalone_platform().ok_or(())?;
        let len: u64 = (self.len() as usize).try_into().unwrap();
        self.set_default_arch(&arch);
        self.set_default_platform(&plat);
        self.add_segment(
            Segment::builder(0x18001c000..0x18001c000+0x166a20)
                //.parent_backing(0x18001c000..0x18001c000+0x166a20)
                .parent_backing(self.parent_view()?.start()..self.parent_view()?.len() as u64)
                .is_auto(true)
                .readable(true)
                .writable(false)
                .executable(true)
                .contains_data(true)
                .contains_code(true),
        );
        info!("added segment");
        self.add_section(Section::builder("iBoot", 0x18001c000..0x18001c000+0x166a20).semantics(Semantics::ReadOnlyCode).is_auto(true));
        self.add_entry_point(&plat, 0x18001c000);
        let start = Symbol::builder(SymbolType::Function, "_start", 0x18001c000).create();
        self.define_auto_symbol(&start);
        Ok(())
    }

}

impl AsRef<BinaryView> for iBootView {
    fn as_ref(&self) -> &BinaryView {
        &self.inner
    }
}

impl BinaryViewBase for iBootView {
    // TODO: This should be filled out with the actual address size
    // from the platform information in the minidump.
    fn address_size(&self) -> usize {
        8
    }

    fn default_endianness(&self) -> Endianness {
        // TODO: This should be filled out with the actual endianness
        // from the platform information in the minidump.
        Endianness::LittleEndian
    }

    fn entry_point(&self) -> u64 {
        // TODO: We should fill this out with a real entry point.
        // This can be done by getting the main module of the minidump
        // with MinidumpModuleList::main_module,
        // then parsing the PE metadata of the main module to find its entry point(s).
        0
    }
}

unsafe impl CustomBinaryView for iBootView {
    type Args = ();

    fn new(handle: &BinaryView, _args: &Self::Args) -> BinaryViewResult<Self> {
        Ok(iBootView::new(handle))
    }

    fn init(&self, _args: Self::Args) -> BinaryViewResult<()> {
        self.init()
    }
}
