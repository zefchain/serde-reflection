// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use maplit::{btreemap, btreeset};
use serde_generate::analyzer;

#[test]
fn test_strongly_connected_components() {
    use analyzer::strongly_connected_components as scc;
    assert_eq!(
        scc(&btreemap! {
            1 => btreeset![2, 3],
            2 => btreeset![],
            3 => btreeset![2],
        }),
        vec![vec![2], vec![3], vec![1]]
    );
    // Cycles and self-loops; unknown children are ignored.
    assert_eq!(
        scc(&btreemap! {
            1 => btreeset![2, 5],
            2 => btreeset![3],
            3 => btreeset![1, 4, 9],
            4 => btreeset![4],
            5 => btreeset![],
        }),
        vec![vec![4], vec![5], vec![1, 2, 3]]
    );
}

#[test]
fn test_topological_sort() {
    use analyzer::best_effort_topological_sort as tsort;
    // 2 nodes, no cycle
    assert_eq!(
        tsort(&btreemap! {
            1 => btreeset![2],
            2 => btreeset![],
        }),
        vec![2, 1]
    );
    // 3 nodes, no cycle
    assert_eq!(
        tsort(&btreemap! {
            1 => btreeset![2, 3],
            2 => btreeset![],
            3 => btreeset![],
        }),
        vec![2, 3, 1]
    );
    assert_eq!(
        tsort(&btreemap! {
            1 => btreeset![2, 3],
            2 => btreeset![3],
            3 => btreeset![],
        }),
        vec![3, 2, 1]
    );
    // Cycles are broken preferably by ignoring edges to nodes with many dependencies.
    // When 1 is larger.
    assert_eq!(
        tsort(&btreemap! {
            1 => btreeset![2, 4, 5, 3],
            2 => btreeset![],
            3 => btreeset![1],
            4 => btreeset![],
            5 => btreeset![],
        }),
        vec![2, /* ignoring edge to 1 */ 3, 4, 5, 1]
    );
    // When 3 is larger
    assert_eq!(
        tsort(&btreemap! {
            1 => btreeset![2, 3],
            2 => btreeset![],
            3 => btreeset![1, 4, 5],
            4 => btreeset![],
            5 => btreeset![],
        }),
        vec![2, /* ignoring edge to 3 */ 1, 4, 5, 3]
    );
}

#[test]
fn test_on_larger_registry() {
    let registry = test_utils::get_registry().unwrap();
    let map = analyzer::get_dependency_map(&registry).unwrap();
    assert_eq!(
        map.get("SerdeData").unwrap(),
        &btreeset!(
            "CStyleEnum",
            "List",
            "NewTypeStruct",
            "OtherTypes",
            "PrimitiveTypes",
            "Struct",
            "Tree",
            "TupleStruct",
            "UnitStruct",
            "SimpleList",
        )
    );

    let vector = analyzer::best_effort_topological_sort(&map);
    assert_eq!(
        vector,
        vec![
            "CStyleEnum",
            "List",
            "NewTypeStruct",
            "Struct",
            "OtherTypes",
            "PrimitiveTypes",
            "SimpleList",
            "Tree",
            "TupleStruct",
            "UnitStruct",
            "SerdeData"
        ]
    );
}
