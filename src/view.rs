use binaryninja::architecture::ArchitectureExt;
use binaryninja::architecture::CoreArchitecture;
use binaryninja::section::{Section, Semantics};
use binaryninja::segment::Segment;
use binaryninja::symbol::Symbol;
use binaryninja::symbol::SymbolType;
use byteorder::{LittleEndian, ReadBytesExt};
use log::{error, info};
use binaryninja::segment::SegmentFlags;

use binaryninja::binary_view::{BinaryView, BinaryViewBase, BinaryViewExt};
use binaryninja::custom_binary_view::{
    BinaryViewType, BinaryViewTypeBase, CustomBinaryView, CustomBinaryViewType, CustomView,
    CustomViewBuilder,
};
use binaryninja::data_buffer::DataBuffer;
use binaryninja::Endianness;

type BinaryViewResult<R> = binaryninja::binary_view::Result<R>;


/// The _iBoot_ binary view type, which the Rust plugin registers with the Binary Ninja core
/// (via `binaryninja::custombinaryview::register_view_type`) as a possible binary view
/// that can be applied to opened binaries.
///
/// If this view type is valid for an opened binary (determined by `is_valid_for`),
/// the Binary Ninja core then uses this view type to create an actual instance of the _iBoot_
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

    fn is_valid_for(&self, data: &BinaryView) -> bool {
        let names = vec!["SecureROM", "AVPBooter", "iBoot", "iBEC", "iBSS"];
        for i in names {
            if data.read_vec(0x200, i.len()) == i.as_bytes() {
                return true;
            }
        }
        false
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

/// An instance of the actual _iBoot_ custom binary view.
/// This contains the main logic to load the memory segments inside a iBoot file into the binary view.
#[allow(non_camel_case_types)]
pub struct iBootView {
    /// The handle to the "real" BinaryView object, in the Binary Ninja core.
    inner: binaryninja::rc::Ref<BinaryView>,
}

use std::str::Utf8Error;
impl iBootView {
    fn new(view: &BinaryView) -> Self {
        iBootView {
            inner: view.to_owned(),
        }
    }

    fn get_iboot_version(&self) -> Result<String, Utf8Error> {
        let mut value = Vec::<u8>::new();
        self.parent_view()
            .expect("lol")
            .read_into_vec(&mut value, 0x286, 0x7a);
        match std::str::from_utf8(&value) {
            Ok(iboot_version) => Ok(iboot_version.to_string()),
            Err(e) => Err(e),
        }
    }

    fn find_base_addr(&self, buf: DataBuffer) -> u64 {
        let mut base_addr_offset: usize = 0x318;

        let iboot_vers: String = match self.get_iboot_version() {
            Ok(iboot_version_str) => iboot_version_str,
            Err(e) => {
                error!("Error getting iBoot version : {e}");
                return 0;
            }
        };

        let v: Vec<&str> = iboot_vers.split('.').collect();
        if v[0].parse::<u64>().unwrap() >= 6603 {
            base_addr_offset = 0x300
        }

        let mut base_addr_buf = &buf.get_data()[base_addr_offset..base_addr_offset + 8];
        base_addr_buf
            .read_u64::<LittleEndian>()
            .unwrap_or_else(|e| {
                error!("Error {e}");
                0
            })
    }

    fn init(&self) -> BinaryViewResult<()> {
        let parent_view = self.parent_view().ok_or(())?;
        let parent_len = parent_view.len();
        let read_buffer = parent_view.read_buffer(0, parent_view.len() as usize)?;
        let arch = CoreArchitecture::by_name("aarch64").ok_or(())?;
        let plat = arch.standalone_platform().ok_or(())?;

        self.set_default_arch(&arch);
        self.set_default_platform(&plat);

        let base_addr = self.find_base_addr(read_buffer);
        info!("Base address at {:#09x}", base_addr);

        let segment_flags = SegmentFlags::new()
            .readable(true)
            .writable(false)
            .executable(true)
            .contains_data(true)
            .contains_code(true);

        self.add_segment(
            Segment::builder(base_addr..base_addr + parent_len)
                .parent_backing(parent_view.start()..parent_view.len())
                .is_auto(true).flags(segment_flags)
        );

        self.add_section(
            Section::builder("iBoot".to_string(), base_addr..base_addr + parent_len)
                .semantics(Semantics::ReadOnlyCode)
                .is_auto(true),
        );
        self.add_entry_point(base_addr);
        let start = Symbol::builder(SymbolType::Function, "_start", base_addr).create();
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
    fn address_size(&self) -> usize {
        8
    }

    fn default_endianness(&self) -> Endianness {
        Endianness::LittleEndian
    }

    fn entry_point(&self) -> u64 {
        0
    }
}

unsafe impl CustomBinaryView for iBootView {
    type Args = ();

    fn new(handle: &BinaryView, _args: &Self::Args) -> BinaryViewResult<Self> {
        Ok(iBootView::new(handle))
    }

    fn init(&mut self, _args: Self::Args) -> BinaryViewResult<()> {
        iBootView::init(self)
    }
}
