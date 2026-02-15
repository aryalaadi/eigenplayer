use crate::core::*;
use std::sync::Arc;

pub fn register_property(core: &mut Core, default_volume: f32, enable_eq: bool) {
    core.add_property("playing", PropertyValue::Bool(false));
    core.add_property("current_track", PropertyValue::String("none".to_string()));
    core.add_property("volume", PropertyValue::Float(default_volume));
    core.add_property("playlist", PropertyValue::StringList(Vec::new()));
    core.add_property("enable_eq", PropertyValue::Bool(enable_eq));
}
