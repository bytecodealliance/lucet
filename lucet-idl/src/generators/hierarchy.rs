use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct HierarchyEntry {
    name: Rc<String>,
    offset: usize,
}

impl HierarchyEntry {
    pub fn new(name: String, offset: usize) -> Self {
        HierarchyEntry {
            name: Rc::new(name),
            offset,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hierarchy(Vec<HierarchyEntry>);

impl Hierarchy {
    pub fn new(name: String, offset: usize) -> Self {
        Hierarchy(vec![HierarchyEntry::new(name, offset)])
    }

    pub fn push(&self, name: String, offset: usize) -> Self {
        let mut cloned = self.clone();
        cloned.0.push(HierarchyEntry::new(name, offset));
        cloned
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn idl_name(&self) -> String {
        self.0
            .iter()
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn fn_name(&self) -> String {
        self.0
            .iter()
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    #[allow(dead_code)]
    pub fn parent_name(&self) -> String {
        let len = self.0.len();
        assert!(len > 1);
        self.0
            .iter()
            .take(len - 1)
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn root_name(&self) -> String {
        self.0
            .iter()
            .take(1)
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn current_offset(&self) -> usize {
        self.0.last().expect("Empty hierarchy").offset
    }
}
