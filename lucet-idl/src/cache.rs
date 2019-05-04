use crate::types::DataTypeId;
use std::collections::HashMap;

/// Cached information for a given structure member
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CachedStructMemberEntry {
    pub offset: usize,
}

/// Cached information for a given type
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedTypeEntry {
    pub type_size: usize,
    pub type_align: usize,
    pub members: Vec<CachedStructMemberEntry>,
}

impl CachedTypeEntry {
    pub fn store_members(&mut self, entries: Vec<CachedStructMemberEntry>) {
        self.members = entries;
    }

    pub fn load_member(&self, member_id: usize) -> Option<&CachedStructMemberEntry> {
        self.members.get(member_id)
    }
}

/// Cache information about a type given its id
#[derive(Clone, Debug, Default)]
pub struct Cache {
    type_map: HashMap<DataTypeId, CachedTypeEntry>,
}

impl Cache {
    pub fn store_type(&mut self, id: DataTypeId, entry: CachedTypeEntry) -> &mut CachedTypeEntry {
        if self.type_map.insert(id, entry).is_some() {
            panic!("Type {:?} had already been cached", id)
        }
        self.type_map.get_mut(&id).unwrap()
    }

    pub fn load_type(&self, id: DataTypeId) -> Option<&CachedTypeEntry> {
        self.type_map.get(&id)
    }
}
