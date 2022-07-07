//! minimum version selection algorithm

use crate::core::{version::PartialVersion};
use crate::util::graph::Graph;
use std::collections::{HashMap};
use std::hash::Hash;

#[derive(PartialEq, Debug)]
pub struct Module<T: Eq + Hash + std::fmt::Debug> {
    name: T,
    version: PartialVersion
}

impl<T: Eq + Hash + std::fmt::Debug> Module<T> {
    pub fn new(name: T, version: PartialVersion) -> Self {
        Self { 
            name: name,
            version: version,
        }
    }
}


/// Traverses the graph
// fn collect_requirement_list(target: Graph<_, _>) -> Vec<Module<_>> {
//     todo!();
// }

// Definition: A module refers to a packages unique identification: NAME+VERSION
// Modules are unique.

// step 1: compute a minimal requirement list
fn compute_minimal_requirement_list<T: Eq + Hash + std::fmt::Debug>(build_list: Vec<Module<T>>) -> Vec<Module<T>> {
    let mut result = Vec::new();
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn comp_min_req() {
        let mods = vec![
            Module::new("A", PartialVersion::new().major(1)),

            Module::new("B", PartialVersion::new().major(1).minor(2)),
            Module::new("D", PartialVersion::new().major(1).minor(3)),
            Module::new("E", PartialVersion::new().major(1).minor(2)),

            Module::new("C", PartialVersion::new().major(1).minor(2)),

            Module::new("C", PartialVersion::new().major(1)), // already covered by 1.2
            Module::new("D", PartialVersion::new().major(1).minor(4)),
            Module::new("E", PartialVersion::new().major(1).minor(2)), // already covered by 1.2

            Module::new("C", PartialVersion::new().major(1).minor(2).patch(4)), // replaces 1.2 to be 1.2.4
        ];
        let mut list = compute_minimal_requirement_list(mods);
        list.sort_by(|a, b| {
            match a.name.cmp(&b.name) {
                std::cmp::Ordering::Equal => a.version.partial_cmp(&b.version).unwrap(),
                std::cmp::Ordering::Less => std::cmp::Ordering::Less,
                std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
            }
        });
        assert_eq!(list, vec![
            Module::new("A", PartialVersion::new().major(1)),
            Module::new("B", PartialVersion::new().major(1).minor(2)),
            Module::new("C", PartialVersion::new().major(1).minor(2).patch(4)),
            Module::new("D", PartialVersion::new().major(1).minor(3)),
            Module::new("D", PartialVersion::new().major(1).minor(4)),
            Module::new("E", PartialVersion::new().major(1).minor(2)),
        ]);
    }
}