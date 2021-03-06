use toml_edit::InlineTable;
use crate::core::vhdl::token::Identifier;

use super::symbol::VHDLSymbol;
use crate::core::vhdl::symbol::VHDLParser;

#[derive(Debug, PartialEq, Hash, Eq)]
pub enum PrimaryUnit {
    Entity(Unit),
    Package(Unit),
    Context(Unit),
    Configuration(Unit),
}

impl PrimaryUnit {
    /// Casts to an identifier. 
    /// 
    /// Currently is safe to unwrap in all instances.
    pub fn as_iden(&self) -> Option<&Identifier> {
        Some(match self {
            Self::Entity(u) => &u.name,
            Self::Package(u) => &u.name,
            Self::Context(u) => &u.name,
            Self::Configuration(u) => &u.name,
        })
    }

    /// Serializes the data into a toml inline table
    pub fn to_toml(&self) -> toml_edit::Value {
        let mut item = toml_edit::Value::InlineTable(InlineTable::new());
        let tbl = item.as_inline_table_mut().unwrap();
        tbl.insert("identifier", toml_edit::value(&self.as_iden().unwrap().to_string()).into_value().unwrap());
        tbl.insert("type", toml_edit::value(&self.to_string()).into_value().unwrap());
        item
    }

    /// Deserializes the data from a toml inline table.
    pub fn from_toml(tbl: &toml_edit::InlineTable) -> Option<Self> {
        let unit = Unit {
            name: Identifier::from_str(tbl.get("identifier")?.as_str()?).unwrap(), 
            symbol: None 
        };
        Some(match tbl.get("type")?.as_str()? {
            "entity" => Self::Entity(unit),
            "package" => Self::Package(unit),
            "context" => Self::Context(unit),
            "configuration" => Self::Configuration(unit),
            _ => return None,
        })
    }
}

impl std::fmt::Display for PrimaryUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Entity(_) => "entity",
            Self::Package(_) => "package",
            Self::Context(_) => "context",
            Self::Configuration(_) => "configuration",
        })
    }
}

#[derive(Debug)]
pub struct Unit {
    name: Identifier,
    symbol: Option<VHDLSymbol>
}

impl std::hash::Hash for Unit {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Unit {}

use std::{collections::HashMap, str::FromStr};

pub fn collect_units(files: &Vec<String>) -> HashMap<PrimaryUnit, String> {
    let mut result = HashMap::new();
    for source_file in files {
        if crate::core::fileset::is_vhdl(&source_file) == true {
            let contents = std::fs::read_to_string(&source_file).unwrap();
            let symbols = VHDLParser::read(&contents).into_symbols();
            // transform into primary design units
            symbols.into_iter().filter_map(|sym| {
                let name = sym.as_iden()?.clone();
                match sym {
                    VHDLSymbol::Entity(_) => Some(PrimaryUnit::Entity(Unit{ name: name, symbol: Some(sym) })),
                    VHDLSymbol::Package(_) => Some(PrimaryUnit::Package(Unit{ name: name, symbol: Some(sym) })),
                    VHDLSymbol::Configuration(_) => Some(PrimaryUnit::Configuration(Unit{ name: name, symbol: Some(sym) })),
                    VHDLSymbol::Context(_) => Some(PrimaryUnit::Context(Unit{ name: name, symbol: Some(sym) })),
                    _ => None,
                }
            }).for_each(|e| {
                result.insert(e, source_file.clone());
            });
        }
    }
    result
}