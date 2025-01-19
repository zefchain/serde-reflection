// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_reflection::{ContainerFormat, Format, FormatHolder, Registry, Result};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Compute dependencies while ignoring external names.
fn get_dependencies<'a>(
    format: &'a ContainerFormat,
    external: &BTreeSet<String>,
) -> Result<BTreeSet<&'a str>> {
    let mut result = BTreeSet::new();
    format.visit(&mut |format| {
        if let Format::TypeName(x) = format {
            if !external.contains(x) {
                result.insert(x.as_str());
            }
        }
        Ok(())
    })?;
    Ok(result)
}

/// Build a map of dependencies between the entries of a `Registry`.
/// * By definition, an entry named `x` depends on `y` iff the container format of `x` in the registry
/// syntactically contains a reference to `y` (i.e. an expression `Format::TypeName(y)`).
/// * Dependencies can play a role in code generation in some languages (e.g. Rust or C++) where inductive
/// definitions may require explicit "boxing" (i.e. adding pointer indirections) to ensure finite object sizes.
pub fn get_dependency_map(registry: &Registry) -> Result<BTreeMap<&str, BTreeSet<&str>>> {
    get_dependency_map_with_external_dependencies(registry, &BTreeSet::new())
}

/// Same as get_dependency_map but allow to specify a set of externally-provided names to ignore.
pub fn get_dependency_map_with_external_dependencies<'a>(
    registry: &'a Registry,
    external: &BTreeSet<String>,
) -> Result<BTreeMap<&'a str, BTreeSet<&'a str>>> {
    let mut children = BTreeMap::new();
    for (name, format) in registry {
        children.insert(name.as_str(), get_dependencies(format, external)?);
    }
    Ok(children)
}

/// Classic topological sorting algorithm except that it doesn't abort in case of cycles.
pub fn best_effort_topological_sort<T>(children: &BTreeMap<T, BTreeSet<T>>) -> Vec<T>
where
    T: Clone + std::cmp::Ord + std::cmp::Eq + std::hash::Hash,
{
    // Build the initial queue so that we pick up nodes with less children first (and otherwise
    // those with smaller key first).
    // This is a heuristic to break cycles preferably at large nodes (see comment below).
    let mut queue: Vec<_> = children.keys().rev().cloned().collect();
    queue.sort_by(|node1, node2| children[node1].len().cmp(&children[node2].len()));

    let mut result = Vec::new();
    // Nodes already inserted in result.
    let mut sorted = HashSet::new();
    // Nodes for which children have been enqueued.
    let mut seen = HashSet::new();

    while let Some(node) = queue.pop() {
        if sorted.contains(&node) {
            continue;
        }
        if seen.contains(&node) {
            // Second time we see this node.
            // * If `node` does not belong to a cycle in the graph, then by now, all its children
            // have been sorted.
            // * If `node` has children that depend back on it. We may be visiting `node` again
            // before some of those children. Just insert `node` here. By ignoring edges going back
            // to `node` now, we are effectively deciding to "break the cycle" there in future
            // applications (e.g. `node` may be forward-declared in C++ and `Box`-ed in Rust).
            sorted.insert(node.clone());
            result.push(node);
            continue;
        }
        // First time we see this node:
        // 1. Mark it so that it is no longer enqueued.
        seen.insert(node.clone());
        // 2. Schedule all the (yet unseen) children then this node for a second visit.
        // (If possible, visit children by increasing key.)
        queue.push(node.clone());
        for child in children[&node].iter().rev() {
            if !seen.contains(child) {
                queue.push(child.clone());
            }
        }
    }
    result
}
