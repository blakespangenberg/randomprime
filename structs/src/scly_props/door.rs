use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::scly_props::structs::{ActorParameters, AncsProp};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Door<'r> {
    #[auto_struct(expect = 14)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub ancs: AncsProp,
    pub actor_params: ActorParameters,//GenericArray<u32, U32>,

    pub scan_offset: GenericArray<f32, U3>,
    pub collision_size: GenericArray<f32, U3>,
    pub collision_offset: GenericArray<f32, U3>,

    pub active: u8,
    pub open: u8,
    pub projectiles_collide: u8,
    pub animation_length: f32,
    pub is_morphball: u8,

}

impl<'r> SclyPropertyData for Door<'r> {
    const OBJECT_TYPE: u8 = 0x03;
}
