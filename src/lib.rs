pub extern crate structs;
extern crate flate2;

pub use structs::reader_writer;
use reader_writer::{LCow, Reader};
use flate2::{Decompress, Flush};

use std::borrow::Cow;

pub mod elevators;
pub mod mlvl_wrapper;
pub mod pickup_meta;

pub trait GcDiscLookupExtensions<'a>
{
    fn find_file(&self, name: &str) -> &structs::FstEntry<'a>;
    fn find_file_mut(&mut self, name: &str) -> &mut structs::FstEntry<'a>;
    fn find_resource<'r, F>(&'r self, pak_name: &str, f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
}

impl<'a> GcDiscLookupExtensions<'a> for structs::GcDisc<'a>
{
    fn find_file(&self, name: &str) -> &structs::FstEntry<'a>
    {
        let fst = &self.file_system_table;
        fst.fst_entries.iter()
            .find(|e| e.name.to_bytes() == name.as_bytes())
            .unwrap()
    }

    fn find_file_mut(&mut self, name: &str) -> &mut structs::FstEntry<'a>
    {
        let fst = &mut self.file_system_table;
        fst.fst_entries.iter_mut()
            .find(|e| e.name.to_bytes() == name.as_bytes())
            .unwrap()
    }

    fn find_resource<'r, F>(&'r self, pak_name: &str, mut f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file(pak_name);
        match *file_entry.file()? {
            structs::FstEntryFile::Pak(ref pak) => pak.resources.iter().find(|res| f(&res)),
            structs::FstEntryFile::Unknown(ref reader) => {
                let pak: structs::Pak = reader.clone().read(());
                pak.resources.iter()
                .find(|res| f(&res))
                .map(|res| LCow::Owned(res.into_owned()))
            },
            _ => panic!(),
        }
    }

    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, mut f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file_mut(pak_name);
        file_entry.guess_kind();
        let pak = match *file_entry.file_mut()? {
            structs::FstEntryFile::Pak(ref mut pak) => pak,
            _ => panic!(),
        };
        let mut cursor = pak.resources.cursor();
        loop {
            if cursor.peek().map(|res| f(&res)).unwrap_or(true) {
                break
            }
            cursor.next();
        }
        cursor.into_value()
    }

}


#[derive(Clone, Debug)]
pub struct ResourceData<'a>
{
    pub is_compressed: bool,
    pub data: Reader<'a>,
}


impl<'a> ResourceData<'a>
{
    pub fn new(res: &structs::Resource<'a>) -> ResourceData<'a>
    {
        let reader = match res.kind {
            structs::ResourceKind::Unknown(ref reader, _) => reader.clone(),
            _ => panic!("Only uninitialized (aka Unknown) resources may be added."),
        };
        ResourceData {
            is_compressed: res.compressed,
            data: reader,
        }
    }
    pub fn decompress(&self) -> Cow<'a, [u8]>
    {
        if self.is_compressed {
            let mut reader = self.data.clone();
            let size: u32 = reader.read(());
            let _header: u16 = reader.read(());
            // TODO: We could use Vec::set_len to avoid initializing the whole array.
            let mut output = vec![0; size as usize];
            Decompress::new(false).decompress(&reader, &mut output, Flush::Finish).unwrap();

            Cow::Owned(output)
        } else {
            Cow::Borrowed(&self.data)
        }
    }
}

macro_rules! def_asset_ids {
    (@Build { $prev:expr } $id:ident, $($rest:tt)*) => {
        def_asset_ids!(@Build { $prev } $id = $prev + 1, $($rest)*);
    };
    (@Build { $_prev:expr } $id:ident = $e:expr, $($rest:tt)*) => {
        pub const $id: u32 = $e;
        def_asset_ids!(@Build { $id } $($rest)*);
    };
    (@Build { $prev:expr }) => {
    };
    ($($tokens:tt)*) => {
        def_asset_ids!(@Build { 0 } $($tokens)*);
    };
}

pub mod asset_ids {
    def_asset_ids! {
        PHAZON_SUIT_SCAN = 0xDEAF0000,
        PHAZON_SUIT_STRG,
        PHAZON_SUIT_TXTR1,
        PHAZON_SUIT_TXTR2,
        PHAZON_SUIT_CMDL,
        PHAZON_SUIT_ANCS,
        NOTHING_ACQUIRED_HUDMEMO_STRG,
        NOTHING_SCAN_STRG, // 0xDEAF0007
        NOTHING_SCAN,
        NOTHING_TXTR,
        NOTHING_CMDL,
        NOTHING_ANCS,

        SKIP_HUDMEMO_STRG_START,
        SKIP_HUDMEMO_STRG_END = SKIP_HUDMEMO_STRG_START + 37,

        GRAVITY_SUIT_CMDL = 0x95946E41,
        GRAVITY_SUIT_ANCS = 0x27A97006,
        PHAZON_SUIT_ACQUIRED_HUDMEMO_STRG = 0x11BEB861,
        PHAZON_MINES_SAVW = 0x2D52090E,
        ARTIFACT_TEMPLE_MREA = 0x2398E906,
    }
}