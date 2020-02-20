use crate::Error;
use serde_json::{self, Map, Value};
use std::collections::{hash_map::Entry, HashMap};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Bindings {
    bindings: HashMap<String, HashMap<String, String>>,
}

impl Bindings {
    pub fn new(bindings: HashMap<String, HashMap<String, String>>) -> Bindings {
        Self { bindings }
    }

    pub fn env(env: HashMap<String, String>) -> Bindings {
        let mut bindings = HashMap::new();
        bindings.insert("env".to_owned(), env);
        Self::new(bindings)
    }

    pub fn empty() -> Bindings {
        Self::new(HashMap::new())
    }

    pub fn from_json(v: &Value) -> Result<Bindings, Error> {
        match v.as_object() {
            Some(modules) => Self::parse_modules_json_obj(modules),
            None => Err(Error::ParseJsonObjError),
        }
    }

    pub fn from_str(s: &str) -> Result<Bindings, Error> {
        let top: Value = serde_json::from_str(s)?;
        Ok(Self::from_json(&top)?)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Bindings, Error> {
        let contents = fs::read_to_string(path.as_ref())?;
        Ok(Self::from_str(&contents)?)
    }

    pub fn extend(&mut self, other: &Bindings) -> Result<(), Error> {
        for (modname, othermodbindings) in other.bindings.iter() {
            match self.bindings.entry(modname.clone()) {
                Entry::Occupied(mut e) => {
                    let existing = e.get_mut();
                    for (bindname, binding) in othermodbindings {
                        match existing.entry(bindname.clone()) {
                            Entry::Vacant(e) => {
                                e.insert(binding.clone());
                            }
                            Entry::Occupied(e) => {
                                if binding != e.get() {
                                    return Err(Error::RebindError {
                                        key: e.key().to_owned(),
                                        binding: binding.to_owned(),
                                        attempt: e.get().to_owned(),
                                    });
                                }
                            }
                        }
                    }
                }
                Entry::Vacant(e) => {
                    e.insert(othermodbindings.clone());
                }
            }
        }
        Ok(())
    }

    pub fn translate(&self, module: &str, symbol: &str) -> Result<&str, Error> {
        match self.bindings.get(module) {
            Some(m) => match m.get(symbol) {
                Some(s) => Ok(s),
                None => Err(Error::UnknownSymbol {
                    module: module.to_owned(),
                    symbol: symbol.to_owned(),
                }),
            },
            None => Err(Error::UnknownModule {
                module: module.to_owned(),
                symbol: symbol.to_owned(),
            }),
        }
    }

    fn parse_modules_json_obj(m: &Map<String, Value>) -> Result<Self, Error> {
        let mut res = HashMap::new();
        for (modulename, values) in m {
            match values.as_object() {
                Some(methods) => {
                    let methodmap = Self::parse_methods_json_obj(methods)?;
                    res.insert(modulename.to_owned(), methodmap);
                }
                None => {
                    return Err(Error::ParseError {
                        key: modulename.to_owned(),
                        value: values.to_string(),
                    });
                }
            }
        }
        Ok(Self::new(res))
    }

    fn parse_methods_json_obj(m: &Map<String, Value>) -> Result<HashMap<String, String>, Error> {
        let mut res = HashMap::new();
        for (method, i) in m {
            match i.as_str() {
                Some(importbinding) => {
                    res.insert(method.to_owned(), importbinding.to_owned());
                }
                None => {
                    return Err(Error::ParseError {
                        key: method.to_owned(),
                        value: i.to_string(),
                    });
                }
            }
        }
        Ok(res)
    }

    pub fn to_string(&self) -> Result<String, Error> {
        let s = serde_json::to_string(&self.to_json())?;
        Ok(s)
    }

    pub fn to_json(&self) -> Value {
        Value::from(self.serialize_modules_json_obj())
    }

    fn serialize_modules_json_obj(&self) -> Map<String, Value> {
        let mut m = Map::new();
        for (modulename, values) in self.bindings.iter() {
            m.insert(
                modulename.to_owned(),
                Value::from(Self::serialize_methods_json_obj(values)),
            );
        }
        m
    }

    fn serialize_methods_json_obj(methods: &HashMap<String, String>) -> Map<String, Value> {
        let mut m = Map::new();
        for (methodname, symbol) in methods.iter() {
            m.insert(methodname.to_owned(), Value::from(symbol.to_owned()));
        }
        m
    }
}
