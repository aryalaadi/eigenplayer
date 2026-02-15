-- Configuration settings via properties
-- These will be set when config.lua is executed after core initialization

if core then
    -- Audio settings
    core:set_property("ring_buffer_size", 88200)
    core:set_property("default_volume", 0.1)
    core:set_property("enable_eq", true)
    core:set_property("eq_bands",{{1000, 1, 1, 1}})

    -- Add more config properties here as needed
end
