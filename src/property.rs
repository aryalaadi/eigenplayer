use crate::core::*;

pub fn register_property(core: &mut Core) {
    // Playback properties
    core.add_property("playing", PropertyValue::Bool(false));
    core.add_property("current_track", PropertyValue::String("none".to_string()));
    core.add_property("volume", PropertyValue::Float(1.0));
    core.add_property("playlist", PropertyValue::StringList(Vec::new()));
    core.add_property("enable_eq", PropertyValue::Bool(false));
    // Config properties - these will be set from config.lua
    core.add_property("ring_buffer_size", PropertyValue::Int(88200));
    core.add_property("default_volume", PropertyValue::Float(0.5));
    core.add_property("eq_bands", PropertyValue::EqBandList(Vec::new()));
}
