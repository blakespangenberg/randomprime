#![recursion_limit = "128"]

pub use structs;
pub use reader_writer;
pub use memmap;

use reader_writer::{
    LCow,
    Reader,
};


use flate2::{Decompress, FlushDecompress};
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;

use std::{
    borrow::Cow,
    collections::hash_map::DefaultHasher,
    ffi::{CStr, CString},
    hash::Hasher,
    iter,
};

pub mod elevators;
pub mod mlvl_wrapper;
pub mod pickup_meta;
pub mod door_meta;
pub mod patcher;
pub mod patches;
pub mod c_interface;
pub mod gcz_writer;
pub mod ciso_writer;
pub mod dol_patcher;

pub trait GcDiscLookupExtensions<'a>
{
    fn find_file(&self, name: &str) -> Option<&structs::FstEntry<'a>>;
    fn find_file_mut(&mut self, name: &str) -> Option<&mut structs::FstEntry<'a>>;
    fn find_resource<'r, F>(&'r self, pak_name: &str, f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool;

    fn add_file(&mut self, path: &str, file: structs::FstEntryFile<'a>) -> Result<(), String>;
}

impl<'a> GcDiscLookupExtensions<'a> for structs::GcDisc<'a>
{
    fn find_file(&self, name: &str) -> Option<&structs::FstEntry<'a>>
    {
        let mut entry = &self.file_system_root;
        for seg in name.split('/') {
            if seg.len() == 0 {
                continue
            }
            match entry {
                structs::FstEntry::Dir(_, entries) => {
                    entry = entries.iter()
                        .find(|e| e.name().to_bytes() == seg.as_bytes())?;
                },
                structs::FstEntry::File(_, _, _) => return None,
            }
        }
        Some(entry)
    }

    fn find_file_mut(&mut self, name: &str) -> Option<&mut structs::FstEntry<'a>>
    {
        let mut entry = &mut self.file_system_root;
        for seg in name.split('/') {
            if seg.len() == 0 {
                continue
            }
            match entry {
                structs::FstEntry::Dir(_, entries) => {
                    entry = entries.iter_mut()
                        .find(|e| e.name().to_bytes() == seg.as_bytes())?;
                },
                structs::FstEntry::File(_, _, _) => return None,
            }
        }
        Some(entry)
    }

    fn find_resource<'r, F>(&'r self, pak_name: &str, mut f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file(pak_name)?;
        match file_entry.file()? {
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
        let file_entry = self.find_file_mut(pak_name)?;
        file_entry.guess_kind();
        let pak = match file_entry.file_mut()? {
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

    fn add_file(&mut self, path: &str, file: structs::FstEntryFile<'a>) -> Result<(), String>
    {
        let mut split = path.rsplitn(2, '/');
        let file_name = split.next()
            .ok_or_else(|| "".to_owned())?;

        let new_entry = structs::FstEntry::File(
            Cow::Owned(CString::new(file_name).unwrap()),
            file,
            None,
        );
        let path = if let Some(path) = split.next() {
            path
        } else {
            self.file_system_root.dir_entries_mut().unwrap().push(new_entry);
            return Ok(())
        };

        let mut entry = &mut self.file_system_root;
        for seg in path.split('/') {
            if seg.len() == 0 {
                continue
            }
            let dir_entries = entry.dir_entries_mut()
                .ok_or_else(|| "".to_owned())?;

            let maybe_pos = dir_entries
                .iter()
                .position(|e| e.name().to_bytes() == seg.as_bytes());
            if let Some(pos) = maybe_pos {
                entry = &mut dir_entries[pos];
            } else {
                dir_entries.push(structs::FstEntry::Dir(
                    Cow::Owned(CString::new(seg).unwrap()),
                    vec![],
                ));
                entry = dir_entries.last_mut().unwrap()
            }
        }

        entry.dir_entries_mut()
            .ok_or_else(|| "".to_owned())?
            .push(new_entry);
        Ok(())
    }
}

pub fn extract_flaahgra_music_files(iso_path: &str) -> Result<[nod_wrapper::FileWrapper; 2], String>
{
    let res = (|| {
        let dw = nod_wrapper::DiscWrapper::new(iso_path)?;
        Ok([
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraR.dsp\0").unwrap())?,
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraL.dsp\0").unwrap())?,
        ])
    })();
    res.map_err(|s: String| format!("Failed to extract Flaahgra music files: {}", s))
}

pub fn parse_layout_chars_to_ints<I>(bytes: &[u8], layout_data_size: usize, checksum_size: usize, is: I)
    -> Result<Vec<u8>, String>
    where I: Iterator<Item = u8> + Clone
{
    const LAYOUT_CHAR_TABLE: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";

    let mut sum: BigUint = 0u8.into();
    for c in bytes.iter().rev() {
        if let Some(idx) = LAYOUT_CHAR_TABLE.iter().position(|i| i == c) {
            sum = sum * BigUint::from(64u8) + BigUint::from(idx);
        } else {
            return Err(format!("Layout contains invalid character '{}'.", c));
        }
    }

    // Reverse the order of the odd bits
    let mut bits = sum.to_str_radix(2).into_bytes();
    for i in 0..(bits.len() / 4) {
        let len = bits.len() - bits.len() % 2;
        bits.swap(i * 2 + 1, len - i * 2 - 1);
    }
    sum = BigUint::parse_bytes(&bits, 2).unwrap();

    // The upper `checksum_size` bits are a checksum, so seperate them from the sum.
    let checksum_bitmask = (1u8 << checksum_size) - 1;
    let checksum = sum.clone() & (BigUint::from(checksum_bitmask) << layout_data_size);
    sum -= checksum.clone();
    let checksum = (checksum >> layout_data_size).to_u8().unwrap();

    let mut computed_checksum = 0;
    {
        let mut sum = sum.clone();
        while sum > 0u8.into() {
            let remainder = (sum.clone() & BigUint::from(checksum_bitmask)).to_u8().unwrap();
            computed_checksum = (computed_checksum + remainder) & checksum_bitmask;
            sum >>= checksum_size;
        }
    }
    if checksum != computed_checksum {
        return Err("Layout checksum failed.".to_string());
    }

    let mut res = vec![];
    for denum in is {
        let (quotient, remainder) = sum.div_rem(&denum.into());
        res.push(remainder.to_u8().unwrap());
        sum = quotient;
    }

    assert!(sum == 0u8.into());

    res.reverse();
    Ok(res)
}


pub fn parse_layout(text: &str) -> Result<(Vec<u8>, Vec<u8>, u64), String>
{
    if !text.is_ascii() {
        return Err("Layout string contains non-ascii characters.".to_string());
    }
    let text = text.as_bytes();

    let (elevator_bytes, pickup_bytes) = if let Some(n) = text.iter().position(|c| *c == b'.') {
        (&text[..n], &text[(n + 1)..])
    } else {
        (b"qzoCAr2fwehJmRjM" as &[u8], text)
    };

    if elevator_bytes.len() != 16 {
        let msg = "The section of the layout string before the '.' should be 16 characters";
        return Err(msg.to_string());
    }

    let (pickup_bytes, has_scan_visor) = if pickup_bytes.starts_with(b"!") {
        (&pickup_bytes[1..], true)
    } else {
        (pickup_bytes, false)
    };
    if pickup_bytes.len() != 87 {
        return Err("Layout string should be exactly 87 characters".to_string());
    }

    // XXX The distribution on this hash probably isn't very good, but we don't use it for anything
    //     particularly important anyway...
    let mut hasher = DefaultHasher::new();
    hasher.write(elevator_bytes);
    hasher.write(pickup_bytes);
    let seed = hasher.finish();

    let pickup_layout = parse_layout_chars_to_ints(
            pickup_bytes,
            if has_scan_visor { 521 } else { 517 },
            if has_scan_visor { 1 } else { 5 },
            iter::repeat(if has_scan_visor { 37u8 } else { 36u8 }).take(100)
        ).map_err(|err| format!("Parsing pickup layout: {}", err))?;

    let elevator_layout = parse_layout_chars_to_ints(
            elevator_bytes,
            91, 5,
            iter::once(21u8).chain(iter::repeat(20u8).take(20))
        ).map_err(|err| format!("Parsing elevator layout: {}", err))?;

    Ok((pickup_layout, elevator_layout, seed))
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
            Decompress::new(false).decompress(&reader, &mut output, FlushDecompress::Finish).unwrap();

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

pub mod custom_asset_ids {
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
        THERMAL_VISOR_SCAN,
        THERMAL_VISOR_STRG,
        SCAN_VISOR_ACQUIRED_HUDMEMO_STRG,
        SCAN_VISOR_SCAN_STRG,
        SCAN_VISOR_SCAN,
        SHINY_MISSILE_TXTR0,
        SHINY_MISSILE_TXTR1,
        SHINY_MISSILE_TXTR2,
        SHINY_MISSILE_CMDL,
        SHINY_MISSILE_ANCS,
        SHINY_MISSILE_EVNT,
        SHINY_MISSILE_ANIM,
        SHINY_MISSILE_ACQUIRED_HUDMEMO_STRG,
        SHINY_MISSILE_SCAN_STRG,
        SHINY_MISSILE_SCAN,

        // Door Variants //
        MORPH_BALL_BOMB_DOOR_CMDL,
        MORPH_BALL_BOMB_DOOR_TXTR,
        POWER_BOMB_DOOR_CMDL,
        POWER_BOMB_DOOR_TXTR,
        MISSILE_DOOR_CMDL,
        MISSILE_DOOR_TXTR,
        CHARGE_DOOR_CMDL,
        CHARGE_DOOR_TXTR,
        SUPER_MISSILE_DOOR_CMDL,
        SUPER_MISSILE_DOOR_TXTR,
        WAVEBUSTER_DOOR_CMDL,
        WAVEBUSTER_DOOR_TXTR,
        ICESPREADER_DOOR_CMDL,
        ICESPREADER_DOOR_TXTR,
        FLAMETHROWER_DOOR_CMDL,
        FLAMETHROWER_DOOR_TXTR,
        DISABLED_DOOR_CMDL,
        DISABLED_DOOR_TXTR,
        AI_DOOR_CMDL,
        AI_DOOR_TXTR,
        
        // Vertical Door Variants //
        VERTICAL_RED_DOOR_CMDL,
        VERTICAL_POWER_BOMB_DOOR_CMDL,
        VERTICAL_MORPH_BALL_BOMB_DOOR_CMDL,
        VERTICAL_MISSILE_DOOR_CMDL,
        VERTICAL_CHARGE_DOOR_CMDL,
        VERTICAL_SUPER_MISSILE_DOOR_CMDL,
        VERTICAL_DISABLED_DOOR_CMDL,
        VERTICAL_WAVEBUSTER_DOOR_CMDL,
        VERTICAL_ICESPREADER_DOOR_CMDL,
        VERTICAL_FLAMETHROWER_DOOR_CMDL,
        VERTICAL_AI_DOOR_CMDL,
        
        // has to be at the end //
        SKIP_HUDMEMO_STRG_START,
        SKIP_HUDMEMO_STRG_END = SKIP_HUDMEMO_STRG_START + 38,
    }
}
