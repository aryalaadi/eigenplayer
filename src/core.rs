use std::collections::HashMap;
use std::sync::Arc;

// Property value types
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    String(String),
    Bool(bool),
    Float(f32),
    StringList(Vec<String>),
}

impl PropertyValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            PropertyValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PropertyValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match self {
            PropertyValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_string_list(&self) -> Option<&Vec<String>> {
        match self {
            PropertyValue::StringList(list) => Some(list),
            _ => None,
        }
    }
}

pub type PropertyCallback = Arc<dyn Fn(&PropertyValue, &Core) + Send + Sync>;

pub struct Property {
    pub value: PropertyValue,
    pub callbacks: Vec<PropertyCallback>,
}

impl Property {
    pub fn new(initial: PropertyValue) -> Self {
        Self {
            value: initial,
            callbacks: Vec::new(),
        }
    }

    pub fn set(&mut self, new_value: PropertyValue) {
        self.value = new_value;
    }

    pub fn get(&self) -> &PropertyValue {
        &self.value
    }

    pub fn subscribe(&mut self, callback: PropertyCallback) {
        self.callbacks.push(callback);
    }
}

pub type CommandCallback = Arc<dyn Fn(Vec<String>, &mut Core) + Send + Sync>;

pub struct Command {
    pub execute: CommandCallback,
}

pub enum EventType {
    PropertyChanged(String),
    CommandExecuted(String),
}

pub type EventCallback = Arc<dyn Fn(&EventType, &Core) + Send + Sync>;

pub struct Core {
    pub properties: HashMap<String, Property>,
    pub commands: HashMap<String, Command>,
    pub event_callbacks: Vec<EventCallback>,
}

impl Core {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            commands: HashMap::new(),
            event_callbacks: Vec::new(),
        }
    }

    pub fn add_property(&mut self, name: &str, value: PropertyValue) {
        self.properties
            .insert(name.to_string(), Property::new(value));
    }

    pub fn set_property(&mut self, name: &str, value: PropertyValue) {
        let prop_callbacks = if let Some(prop) = self.properties.get_mut(name) {
            prop.set(value.clone());
            prop.callbacks.clone()
        } else {
            return;
        };

        for cb in &prop_callbacks {
            cb(&value, self);
        }

        let event = EventType::PropertyChanged(name.to_string());

        for cb in &self.event_callbacks {
            cb(&event, self);
        }
    }

    pub fn get_property(&self, name: &str) -> Option<&PropertyValue> {
        self.properties.get(name).map(|p| p.get())
    }

    // Typed getters for convenience
    pub fn get_string(&self, name: &str) -> Option<&String> {
        self.get_property(name).and_then(|v| v.as_string())
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get_property(name).and_then(|v| v.as_bool())
    }

    pub fn get_float(&self, name: &str) -> Option<f32> {
        self.get_property(name).and_then(|v| v.as_float())
    }

    pub fn get_string_list(&self, name: &str) -> Option<&Vec<String>> {
        self.get_property(name).and_then(|v| v.as_string_list())
    }

    pub fn add_command(&mut self, name: &str, command: Command) {
        self.commands.insert(name.to_string(), command);
    }

    pub fn execute_command(&mut self, name: &str, params: Vec<String>) {
        if let Some(cmd) = self.commands.get(name) {
            let exec_fn = Arc::clone(&cmd.execute);
            exec_fn(params, self);
        }

        let event = EventType::CommandExecuted(name.to_string());
        for cb in &self.event_callbacks {
            cb(&event, self);
        }
    }

    pub fn subscribe_event(&mut self, callback: EventCallback) {
        self.event_callbacks.push(callback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_value_types() {
        let str_val = PropertyValue::String("test".to_string());
        assert_eq!(str_val.as_string(), Some(&"test".to_string()));
        assert_eq!(str_val.as_bool(), None);

        let bool_val = PropertyValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_string(), None);

        let float_val = PropertyValue::Float(0.5);
        assert_eq!(float_val.as_float(), Some(0.5));

        let list_val = PropertyValue::StringList(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            list_val.as_string_list(),
            Some(&vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn test_core_properties() {
        let mut core = Core::new();

        core.add_property("playing", PropertyValue::Bool(false));
        core.add_property("volume", PropertyValue::Float(0.5));
        core.add_property("playlist", PropertyValue::StringList(vec![]));

        assert_eq!(core.get_bool("playing"), Some(false));
        assert_eq!(core.get_float("volume"), Some(0.5));
        assert_eq!(core.get_string_list("playlist"), Some(&vec![]));

        core.set_property("playing", PropertyValue::Bool(true));
        assert_eq!(core.get_bool("playing"), Some(true));
    }

    #[test]
    fn test_property_callbacks() {
        let mut core = Core::new();
        core.add_property("test", PropertyValue::String("initial".to_string()));

        let callback_triggered = Arc::new(std::sync::Mutex::new(false));
        let callback_triggered_clone = Arc::clone(&callback_triggered);

        if let Some(prop) = core.properties.get_mut("test") {
            prop.subscribe(Arc::new(move |_val, _core| {
                *callback_triggered_clone.lock().unwrap() = true;
            }));
        }

        core.set_property("test", PropertyValue::String("changed".to_string()));
        assert!(*callback_triggered.lock().unwrap());
    }

    #[test]
    fn test_commands() {
        let mut core = Core::new();
        core.add_property("value", PropertyValue::String("initial".to_string()));

        core.add_command(
            "set_value",
            Command {
                execute: Arc::new(|params, core| {
                    if let Some(val) = params.get(0) {
                        core.set_property("value", PropertyValue::String(val.clone()));
                    }
                }),
            },
        );

        core.execute_command("set_value", vec!["new_value".to_string()]);
        assert_eq!(core.get_string("value"), Some(&"new_value".to_string()));
    }
}
