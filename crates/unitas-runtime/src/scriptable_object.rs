macro_rules! ffi_export {
    (pub extern "C" fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block) => {
        #[unsafe(export_name = stringify!($name))]
        pub extern "C" fn $name($($arg : $arg_ty),*) $(-> $ret)? {
            #[used]
            static KEEP: extern "C" fn($($arg_ty),*) $(-> $ret)? = $name;
            $body
        }
    };
}

use std::collections::BTreeMap;
use std::sync::Mutex;

use unitas_core::object::{ObjectKind, WorldObject};

struct ScriptableObjectWorld {
    objects: Vec<WorldObject>,
    next_id: u64,
    name_buffer: Vec<u8>,
}

impl ScriptableObjectWorld {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            next_id: 1,
            name_buffer: Vec::new(),
        }
    }

    fn create(&mut self, name: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.objects
            .push(WorldObject::new(id, ObjectKind::ScriptableObject, name));
        id
    }

    fn destroy(&mut self, id: u64) -> bool {
        match self.objects.iter_mut().find(|object| object.id() == id) {
            Some(object) => object.destroy(),
            None => false,
        }
    }

    fn is_alive(&self, id: u64) -> bool {
        match self.objects.iter().find(|object| object.id() == id) {
            Some(object) => !object.is_destroyed(),
            None => false,
        }
    }

    fn name_buffer_address(&mut self, capacity: usize) -> u64 {
        if self.name_buffer.len() < capacity {
            self.name_buffer.resize(capacity, 0);
        }
        self.name_buffer.as_ptr() as usize as u64
    }

    fn read_name(&self, address: u64, length: usize) -> Option<String> {
        if address == 0 || address != self.name_buffer.as_ptr() as usize as u64 {
            return None;
        }
        if length > self.name_buffer.len() {
            return None;
        }
        match std::str::from_utf8(&self.name_buffer[..length]) {
            Ok(text) => Some(text.trim_end_matches('\0').to_string()),
            Err(_) => None,
        }
    }
}

static SCRIPTABLE_OBJECT_WORLDS: Mutex<BTreeMap<u64, ScriptableObjectWorld>> =
    Mutex::new(BTreeMap::new());

ffi_export! {
pub extern "C" fn unitas_runtime_scriptable_object_name_buffer(handle: u64, capacity: u64) -> u64 {
    let cap = match usize::try_from(capacity) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let mut worlds = match SCRIPTABLE_OBJECT_WORLDS.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let world = worlds
        .entry(handle)
        .or_insert_with(ScriptableObjectWorld::new);
    world.name_buffer_address(cap)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_scriptable_object_create(
    handle: u64,
    address: u64,
    length: u64,
) -> u64 {
    let len = match usize::try_from(length) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    let mut worlds = match SCRIPTABLE_OBJECT_WORLDS.lock() {
        Ok(guard) => guard,
        Err(_) => return 0,
    };
    let world = worlds
        .entry(handle)
        .or_insert_with(ScriptableObjectWorld::new);
    let name = match world.read_name(address, len) {
        Some(text) => text,
        None => return 0,
    };
    world.create(name)
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_scriptable_object_destroy(handle: u64, instance: u64) -> i32 {
    if instance == 0 {
        return -2;
    }
    let mut worlds = match SCRIPTABLE_OBJECT_WORLDS.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    let world = worlds
        .entry(handle)
        .or_insert_with(ScriptableObjectWorld::new);
    world.destroy(instance);
    0
}
}

ffi_export! {
pub extern "C" fn unitas_runtime_scriptable_object_is_alive(handle: u64, instance: u64) -> i32 {
    if instance == 0 {
        return 0;
    }
    let worlds = match SCRIPTABLE_OBJECT_WORLDS.lock() {
        Ok(guard) => guard,
        Err(_) => return -1,
    };
    match worlds.get(&handle) {
        Some(world) => i32::from(world.is_alive(instance)),
        None => 0,
    }
}
}
