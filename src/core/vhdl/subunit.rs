use super::{symbol::{self, ResReference}, token::Identifier};

#[derive(Debug, PartialEq)]
pub enum SubUnit {
    Configuration(symbol::Configuration),
    Architecture(symbol::Architecture),
}

impl SubUnit {
    pub fn from_arch(arch: symbol::Architecture) -> Self {
        Self::Architecture(arch)
    }

    pub fn from_config(cfg: symbol::Configuration) -> Self {
        Self::Configuration(cfg)
    }

    pub fn get_edges(&self) -> &Vec<Identifier> {
        match self {
            Self::Architecture(u) => u.edges(),
            Self::Configuration(u) => u.edges(),
        }
    }

    pub fn get_entity(&self) -> &Identifier {
        match self {
            Self::Architecture(u) => u.entity(),
            Self::Configuration(u) => u.entity(),
        }
    }

    pub fn get_refs(&self) -> Vec<&ResReference> {
        match self {
            Self::Architecture(u) => u.get_refs(),
            Self::Configuration(u) => u.get_refs(),
        }
    }
}