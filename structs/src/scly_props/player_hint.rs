
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct PlayerHintStruct
    {
        #[expect = 15]
        prop_count: u32,

        // 15 unknowns, left out for simplicity
        unknowns: GenericArray<u8, U15>,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct PlayerHint<'a>
    {
        #[expect = 6]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,

        unknown0: u8,

        inner_struct: PlayerHintStruct,

        unknown1: u32,
    }
}

impl<'a> SclyPropertyData for PlayerHint<'a>
{
    fn object_type() -> u8
    {
        0x3E
    }
}