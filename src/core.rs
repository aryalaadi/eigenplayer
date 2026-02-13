use std::collections::HashMap;
use std::sync::Arc;

pub type PropertyCallback<T> = Arc<dyn Fn(&T, &Core) + Send + Sync>;

pub struct Property<T> {
    pub value: T,
    pub callbacks: Vec<PropertyCallback<T>>
}

impl<T: Clone> Property<T> {
    pub fn new(initial: T) -> Self {
	Self {
	    value: initial,
	    callbacks: Vec::new(),
	}
    }

    pub fn set(&mut self, new_value: T) {
	self.value = new_value.clone();
    }

    pub fn get(&self) -> &T {
	&self.value
    }

    pub fn subscribe(&mut self, callback: PropertyCallback<T>) {
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
    pub properties: HashMap<String, Property<String>>,
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

    pub fn add_property(&mut self, name: &str, value: String) {
	self.properties.insert(name.to_string(), Property::new(value));
    }

    pub fn set_property(&mut self, name: &str, value: String) {
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

    pub fn get_property(&self, name: &str) -> Option<&String> {
	self.properties.get(name).map(|p| p.get())
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
