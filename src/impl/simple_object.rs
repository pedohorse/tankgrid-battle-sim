use std::sync::atomic::Ordering;

use crate::log_data::LogRepresentable;
use crate::map_object::MapObject;
use crate::script_repr::{self, ToScriptRepr};

use super::unique_id_counter::NEXT_OBJID;

#[derive(Clone, Copy)]
pub enum ObjectCacheType {
    Player(usize),
    AmmoCrate(u64),
    //Stuff, // TODO: add stuff like pickable items
}

impl LogRepresentable for ObjectCacheType {
    fn log_repr(&self) -> String {
        match self {
            ObjectCacheType::Player(_) => "player",
            ObjectCacheType::AmmoCrate(_) => "ammocrate",
        }
        .to_owned()
    }
}

pub struct SimpleObject<R> {
    pub uid: u64,
    pub obj_type: ObjectCacheType,
    pub pos: (i64, i64),
    pub rot: R,
    pub seethroughable: bool,
    pub passable: bool,
    pub shootable: bool,
    pub script_repr: String,
}

impl<R> MapObject<R> for SimpleObject<R>
where
    R: Copy,
{
    fn clone_with_uid(source: &Self, new_uid: u64) -> Self {
        SimpleObject {
            uid: new_uid,
            obj_type: source.obj_type,
            pos: source.pos,
            rot: source.rot,
            seethroughable: source.seethroughable,
            passable: source.passable,
            shootable: source.shootable,
            script_repr: source.script_repr.clone(),
        }
    }

    fn unique_id(&self) -> u64 {
        self.uid
    }

    fn orientation(&self) -> R {
        self.rot
    }

    fn position(&self) -> (i64, i64) {
        self.pos
    }

    fn passable(&self) -> bool {
        self.passable
    }

    fn seethroughable(&self) -> bool {
        self.seethroughable
    }
}

impl<R> ToScriptRepr for SimpleObject<R> {
    fn to_script_repr(&self) -> String {
        self.script_repr.clone()
    }
}

impl<R> LogRepresentable for SimpleObject<R> {
    fn log_repr(&self) -> String {
        format!("{}({})", self.obj_type.log_repr(), self.uid)
    }
}

impl<R> SimpleObject<R> {
    pub fn new(
        col: i64,
        row: i64,
        orientation: R,
        obj_type: ObjectCacheType,
        seethroughable: bool,
        passable: bool,
        shootable: bool,
    ) ->SimpleObject<R> {
        let script_repr = format!("{}", obj_type.log_repr());
        SimpleObject {
            uid: NEXT_OBJID.fetch_add(1, Ordering::Relaxed),
            pos: (col, row),
            rot: orientation,
            obj_type,
            seethroughable,
            passable,
            shootable,
            script_repr,
        }
    }
}