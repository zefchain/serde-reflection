// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_reflection::{ContainerFormat, Error, Format, FormatHolder, Named, VariantFormat};
use std::{collections::HashSet, ops::Deref};

#[test]
fn test_format_visiting() {
    use Format::*;

    let format = ContainerFormat::Enum(
        vec![(
            0,
            Named {
                name: "foo".into(),
                value: VariantFormat::Tuple(vec![
                    TypeName("foo".into()),
                    TypeName("bar".into()),
                    Seq(Box::new(TypeName("foo".into()))),
                ]),
            },
        )]
        .into_iter()
        .collect(),
    );
    let mut names = HashSet::new();
    format
        .visit(&mut |f| {
            if let TypeName(x) = f {
                // Insert a &str borrowed from `format`.
                names.insert(x.as_str());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(names.len(), 2);

    assert!(VariantFormat::unknown().visit(&mut |_| Ok(())).is_err());
    assert!(Format::unknown().visit(&mut |_| Ok(())).is_err());
}

// Note: this does not test pointer equality, only referenced content.
fn assert_variable_contains_value(format: &Format, value: &Format) {
    match format {
        Format::Variable(variable) => {
            assert_eq!(
                variable
                    .borrow()
                    .deref()
                    .as_ref()
                    .expect("must contain a value"),
                value
            );
        }
        _ => panic!(),
    }
}

#[test]
fn test_pattern_variable_unification() {
    let mut x = Format::unknown();
    let mut y = Format::unknown();
    let mut z = Format::unknown();
    x.unify(y.clone()).unwrap();
    // x is untouched when unifying with y.
    // We chose to assign y to (a clone of the RC pointer to the refcell of) x.
    assert_eq!(x, Format::unknown());
    assert_variable_contains_value(&y, &x);

    x.unify(Format::U8).unwrap();
    // y is untouched when assigning x alone.
    assert_variable_contains_value(&y, &x);

    y.unify(z.clone()).unwrap();
    // We chose to assign z.
    assert_variable_contains_value(&y, &x);
    assert_variable_contains_value(&z, &y);

    z.unify(Format::U8).unwrap();
    // x is unchanged
    assert_variable_contains_value(&x, &Format::U8);
    // The clone of y used in z was simplified but not y itself.
    assert_variable_contains_value(&y, &x);
    // z was simplified to use the same refcell as x.
    assert_variable_contains_value(&z, &Format::U8);

    // Re-assigning manually x to confirm.
    match &x {
        Format::Variable(variable) => {
            *variable.borrow_mut() = Some(Format::U16);
        }
        _ => panic!(),
    }
    assert_variable_contains_value(&z, &Format::U16);

    z.reduce();
    // We copied out the value of x into z, which is no longer a variable.
    assert_eq!(z, Format::U16);
    // Not touching x.
    assert_ne!(x, Format::U16);

    y.reduce();
    assert_eq!(y, Format::U16);
    assert_ne!(x, Format::U16);

    x.reduce();
    assert_eq!(x, Format::U16);
}

#[test]
fn test_general_variable_unification() {
    let mut x = Format::unknown();
    let mut y = Format::unknown();
    y.unify(Format::U8).unwrap();
    x.unify(y.clone()).unwrap();
    assert!(x.unify(Format::U16).is_err());
    x.unify(Format::U8).unwrap();

    let mut x = VariantFormat::unknown();
    let mut y = VariantFormat::unknown();
    y.unify(VariantFormat::Unit).unwrap();
    x.unify(y.clone()).unwrap();
    assert!(x
        .unify(VariantFormat::NewType(Box::new(Format::U16)))
        .is_err());
    x.unify(VariantFormat::Unit).unwrap();
}

#[test]
fn test_format_unification() {
    use Format::*;

    let mut x = Format::unknown();
    assert!(x.unify(U8).is_ok());
    x.reduce();
    assert_eq!(x, U8);
    assert_eq!(
        x.unify(U16).unwrap_err(),
        Error::Incompatible("U8".into(), "U16".into())
    );

    let mut x = Tuple(vec![Format::unknown(), U32]);
    x.unify(Tuple(vec![U16, Format::unknown()])).unwrap();
    x.reduce();
    assert_eq!(x, Tuple(vec![U16, U32]));

    for x in vec![
        Unit,
        Bool,
        I8,
        I16,
        I32,
        I64,
        I128,
        U8,
        U16,
        U32,
        U64,
        U128,
        F32,
        F64,
        Char,
        Str,
        Bytes,
        TypeName("foo".into()),
        Option(Box::new(Unit)),
        Seq(Box::new(Unit)),
        Map {
            key: Box::new(Unit),
            value: Box::new(Unit),
        },
        Tuple(vec![Format::unknown()]),
    ]
    .iter_mut()
    {
        assert!(x.unify(TypeName("bar".into())).is_err());
        assert!(x.unify(Option(Box::new(U32))).is_err());
        assert!(x.unify(Seq(Box::new(U32))).is_err());
        assert!(x.unify(Tuple(vec![])).is_err());
    }
}

#[test]
fn test_container_format_unification() {
    use ContainerFormat::*;
    use Format::*;

    let mut x = TupleStruct(vec![Format::unknown(), U32]);
    x.unify(TupleStruct(vec![U16, Format::unknown()])).unwrap();
    x.reduce();
    assert_eq!(x, TupleStruct(vec![U16, U32]));

    let mut x = Enum(
        vec![(
            0,
            Named {
                name: "foo".into(),
                value: VariantFormat::Tuple(vec![Format::unknown()]),
            },
        )]
        .into_iter()
        .collect(),
    );
    assert!(x
        .unify(Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::Unit,
                }
            )]
            .into_iter()
            .collect()
        ))
        .is_err());
    assert!(x
        .unify(Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::Tuple(vec![U8]),
                }
            )]
            .into_iter()
            .collect()
        ))
        .is_ok());

    for x in [
        UnitStruct,
        NewTypeStruct(Box::new(Unit)),
        TupleStruct(vec![Format::unknown()]),
        Struct(vec![Named {
            name: "foo".into(),
            value: Format::unknown(),
        }]),
        Enum(
            vec![(
                0,
                Named {
                    name: "foo".into(),
                    value: VariantFormat::unknown(),
                },
            )]
            .into_iter()
            .collect(),
        ),
    ]
    .iter_mut()
    {
        assert!(x.unify(NewTypeStruct(Box::new(U8))).is_err());
        assert!(x.unify(TupleStruct(vec![])).is_err());
        assert!(x
            .unify(Struct(vec![Named {
                name: "bar".into(),
                value: Format::unknown()
            }]))
            .is_err());
        assert!(x
            .unify(Enum(
                vec![(
                    0,
                    Named {
                        name: "bar".into(),
                        value: VariantFormat::unknown()
                    }
                )]
                .into_iter()
                .collect()
            ))
            .is_err());
    }
}
