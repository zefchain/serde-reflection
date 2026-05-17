use crate::solidity_generation::{get_bytecode, get_registry_from_type};
use alloy_sol_types::sol;
use alloy_sol_types::SolCall as _;
use revm::{
    context::result::{ExecutionResult, Output},
    database::{CacheDB, EmptyDB},
    primitives::{Address, Bytes, TxKind, U256},
    Context, ExecuteCommitEvm, MainBuilder, MainContext,
};
use serde::{
    de::DeserializeOwned,
    {Deserialize, Serialize},
};
use serde_generate::{solidity, CodeGeneratorConfig};
use std::{fmt::Display, fs::File, io::Write};
use tempfile::tempdir;

fn nonce(database: &CacheDB<EmptyDB>, addr: &Address) -> u64 {
    database
        .cache
        .accounts
        .get(addr)
        .map_or(0, |info| info.info.nonce)
}

fn test_contract(bytecode: Bytes, encoded_args: Bytes) {
    let mut database = CacheDB::new(EmptyDB::default());
    let deployer = Address::ZERO;
    let contract_address = {
        let deploy_nonce = nonce(&database, &deployer);
        let result = Context::mainnet()
            .with_db(&mut database)
            .modify_cfg_chained(|cfg| {
                cfg.limit_contract_code_size = Some(usize::MAX);
            })
            .modify_tx_chained(|tx| {
                tx.caller = deployer;
                tx.nonce = deploy_nonce;
                tx.kind = TxKind::Create;
                tx.data = bytecode;
                tx.gas_limit = u64::MAX;
                tx.value = U256::ZERO;
            })
            .build_mainnet()
            .replay_commit()
            .unwrap();

        let ExecutionResult::Success { output, .. } = result else {
            panic!("The TxKind::Create execution failed");
        };
        let Output::Create(_, Some(contract_address)) = output else {
            panic!("Failure to create the contract");
        };
        contract_address
    };

    let call_nonce = nonce(&database, &deployer);
    let result = Context::mainnet()
        .with_db(&mut database)
        .modify_cfg_chained(|cfg| {
            cfg.limit_contract_code_size = Some(usize::MAX);
        })
        .modify_tx_chained(|tx| {
            tx.caller = deployer;
            tx.nonce = call_nonce;
            tx.kind = TxKind::Call(contract_address);
            tx.data = encoded_args;
            tx.gas_limit = u64::MAX;
            tx.value = U256::ZERO;
        })
        .build_mainnet()
        .replay_commit()
        .unwrap();

    let ExecutionResult::Success { .. } = result else {
        panic!("The TxKind::Call execution failed");
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TestVec<T> {
    pub vec: Vec<T>,
}

fn test_vector_serialization<T: Serialize + DeserializeOwned + Display>(
    t: TestVec<T>,
) -> anyhow::Result<()> {
    let registry = get_registry_from_type::<TestVec<T>>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The generated code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        let len = t.vec.len();
        let first_val = &t.vec[0];
        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_deserialization(bytes calldata input) external {{
      Library.TestVec memory t = Library.bcs_deserialize_TestVec(input);
      require(t.vec.length == {len}, "The length is incorrect");
      require(t.vec[0] == {first_val}, "incorrect value");

      bytes memory input_rev = Library.bcs_serialize_TestVec(t);
      require(input.length == input_rev.length);
      for (uint256 i=0; i<input.length; i++) {{
        require(input[i] == input_rev[i]);
      }}
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    // Building the test entry
    let expected_input = bcs::to_bytes(&t).unwrap();

    // Building the input to the smart contract
    sol! {
      function test_deserialization(bytes calldata input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode, fct_args);
    Ok(())
}

#[test]
fn test_vector_serialization_types() {
    let mut vec = vec![0_u16; 3];
    vec[0] = 42;
    vec[1] = 5;
    vec[2] = 360;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_u8; 2];
    vec[0] = 42;
    vec[1] = 5;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_u32; 2];
    vec[0] = 42;
    vec[1] = 5;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_i8; 2];
    vec[0] = -42;
    vec[1] = 76;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_i16; 2];
    vec[0] = -4200;
    vec[1] = 7600;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_i32; 2];
    vec[0] = -4200;
    vec[1] = 7600;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();

    let mut vec = vec![0_i64; 140];
    vec[0] = -4200;
    vec[1] = 7600;
    let t = TestVec { vec };
    test_vector_serialization(t).unwrap();
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SimpleEnumTestType {
    ChoiceA,
    ChoiceB,
    ChoiceC,
}

#[test]
fn test_simple_enum_serialization() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<SimpleEnumTestType>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The generated code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_deserialization(bytes calldata input) external {{
      require(input.length == 1);
      Library.SimpleEnumTestType t = Library.bcs_deserialize_SimpleEnumTestType(input);
      require(t == Library.SimpleEnumTestType.ChoiceB);

      bytes memory input_rev = Library.bcs_serialize_SimpleEnumTestType(t);
      require(input_rev.length == 1);
      require(input[0] == input_rev[0]);
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    // Building the test entry
    let t = SimpleEnumTestType::ChoiceB;
    let expected_input = bcs::to_bytes(&t).unwrap();

    // Building the input to the smart contract
    sol! {
      function test_deserialization(bytes calldata input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode, fct_args);
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StructBoolString {
    a: bool,
    b: String,
}

#[test]
fn test_struct_bool_string() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<StructBoolString>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The generated code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_deserialization(bytes calldata input) external {{
      Library.StructBoolString memory t = Library.bcs_deserialize_StructBoolString(input);

      bytes memory input_rev = Library.bcs_serialize_StructBoolString(t);
      require(input.length == input_rev.length);
      for (uint256 i=0; i<input.length; i++) {{
        require(input[i] == input_rev[i]);
      }}
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    // Building the test entry
    let t = StructBoolString {
        a: false,
        b: "abc".to_string(),
    };
    let expected_input = bcs::to_bytes(&t).unwrap();

    // Building the input to the smart contract
    sol! {
      function test_deserialization(bytes calldata input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode, fct_args);
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ComplexEnumTestType {
    ChoiceA,
    Name(String),
    Age(i32),
}

#[test]
fn test_complex_enum() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<ComplexEnumTestType>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The generated code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_deserialization(bytes calldata input) external {{
      Library.ComplexEnumTestType memory t = Library.bcs_deserialize_ComplexEnumTestType(input);

      bytes memory input_rev = Library.bcs_serialize_ComplexEnumTestType(t);
      require(input.length == input_rev.length);
      for (uint256 i=0; i<input.length; i++) {{
        require(input[i] == input_rev[i]);
      }}
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    // Building the test entry
    let t1 = ComplexEnumTestType::ChoiceA;
    let t2 = ComplexEnumTestType::Name("joe".to_string());
    let t3 = ComplexEnumTestType::Age(43);
    for t in [t1, t2, t3] {
        let expected_input = bcs::to_bytes(&t).unwrap();

        // Building the input to the smart contract
        sol! {
            function test_deserialization(bytes calldata input);
        }
        let input = Bytes::copy_from_slice(&expected_input);
        let fct_args = test_deserializationCall { input };
        let fct_args = fct_args.abi_encode().into();

        test_contract(bytecode.clone(), fct_args);
    }
    Ok(())
}

/// Regression test for ULEB128-encoded variant indices: an enum with >= 128
/// variants forces the discriminant to use more than one byte, exercising the
/// fix that made `choice` a `uint64` encoded/decoded via `bcs_serialize_len` /
/// `bcs_deserialize_offset_len` instead of a single `uint8` byte.
#[test]
fn test_enum_uleb128_variant_index() -> anyhow::Result<()> {
    use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
    use std::collections::BTreeMap;

    // 199 Unit variants + 1 NewType(u32) variant at index 199. The non-trivial
    // tail forces the complex (struct-backed) Enum codepath in the generator.
    let mut variants: BTreeMap<u32, Named<VariantFormat>> = BTreeMap::new();
    for i in 0..199u32 {
        variants.insert(
            i,
            Named {
                name: format!("V{i}"),
                value: VariantFormat::Unit,
            },
        );
    }
    variants.insert(
        199,
        Named {
            name: "WithPayload".into(),
            value: VariantFormat::NewType(Box::new(Format::U32)),
        },
    );

    let mut registry = Registry::new();
    registry.insert("BigEnum".into(), ContainerFormat::Enum(variants));

    let dir = tempdir().unwrap();
    let path = dir.path();

    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let config = CodeGeneratorConfig::new("Library".to_string());
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;
        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_round_trip(bytes calldata input, uint64 expected_choice) external {{
        Library.BigEnum memory t = Library.bcs_deserialize_BigEnum(input);
        require(t.choice == expected_choice, "wrong choice");
        bytes memory input_rev = Library.bcs_serialize_BigEnum(t);
        require(input.length == input_rev.length, "length mismatch");
        for (uint256 i=0; i<input.length; i++) {{
            require(input[i] == input_rev[i], "byte mismatch");
        }}
    }}

}}
"#
        )?;
    }

    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    sol! {
        function test_round_trip(bytes calldata input, uint64 expected_choice);
    }

    // (encoded BCS bytes, expected variant index).
    // - 0:   single-byte ULEB128
    // - 127: largest single-byte ULEB128
    // - 128: smallest two-byte ULEB128 (exercises continuation)
    // - 199: two-byte ULEB128 + u32 payload (the NewType variant)
    let cases: Vec<(Vec<u8>, u64)> = vec![
        (vec![0x00], 0),
        (vec![0x7f], 127),
        (vec![0x80, 0x01], 128),
        (
            {
                let mut v = vec![0xc7, 0x01];
                v.extend_from_slice(&0xdead_beef_u32.to_le_bytes());
                v
            },
            199,
        ),
    ];

    for (input_bytes, expected_choice) in cases {
        let input = Bytes::copy_from_slice(&input_bytes);
        let fct_args = test_round_tripCall {
            input,
            expected_choice,
        };
        let fct_args = fct_args.abi_encode().into();
        test_contract(bytecode.clone(), fct_args);
    }

    Ok(())
}

/// Regression test for sparse Serde variant indices.
///
/// `ContainerFormat::Enum` is a `BTreeMap<u32, ...>`, so the variant indices
/// are not required to be contiguous `0..N-1`. The generator must preserve
/// each variant's original index in the BCS encoding (rather than re-numbering
/// them by position). This test checks dispatch, validation, and the
/// precomputed multi-byte ULEB128 discriminant for a NewType variant at a
/// sparse, multi-byte index.
#[test]
fn test_enum_sparse_variant_indices() -> anyhow::Result<()> {
    use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
    use std::collections::BTreeMap;

    let mut variants: BTreeMap<u32, Named<VariantFormat>> = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Zero".into(),
            value: VariantFormat::Unit,
        },
    );
    variants.insert(
        5,
        Named {
            name: "Five".into(),
            value: VariantFormat::Unit,
        },
    );
    variants.insert(
        128,
        Named {
            name: "OneTwentyEight".into(),
            value: VariantFormat::Unit,
        },
    );
    variants.insert(
        300,
        Named {
            name: "WithPayload".into(),
            value: VariantFormat::NewType(Box::new(Format::U32)),
        },
    );

    let mut registry = Registry::new();
    registry.insert("Sparse".into(), ContainerFormat::Enum(variants));

    let dir = tempdir().unwrap();
    let path = dir.path();

    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let config = CodeGeneratorConfig::new("Library".to_string());
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // Spot-check the generated source: discriminant 300 = ULEB128 0xac 0x02.
    let generated = std::fs::read_to_string(&test_library_path)?;
    assert!(
        generated.contains(r#"hex"ac02""#),
        "generator should embed the precomputed ULEB128 for index 300 (0xac 0x02):\n{generated}"
    );
    // Index 5 collapses to a single byte 0x05.
    assert!(
        generated.contains(r#"hex"05""#),
        "generator should embed the precomputed ULEB128 for index 5 (0x05):\n{generated}"
    );
    // The deserializer must validate against the actual sparse index set,
    // not `choice < N`.
    assert!(
        generated.contains("choice == 0 || choice == 5 || choice == 128 || choice == 300"),
        "deserializer should validate against the sparse index set:\n{generated}"
    );

    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;
        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_round_trip(bytes calldata input, uint64 expected_choice) external {{
        Library.Sparse memory t = Library.bcs_deserialize_Sparse(input);
        require(t.choice == expected_choice, "wrong choice");
        bytes memory input_rev = Library.bcs_serialize_Sparse(t);
        require(input.length == input_rev.length, "length mismatch");
        for (uint256 i=0; i<input.length; i++) {{
            require(input[i] == input_rev[i], "byte mismatch");
        }}
    }}

}}
"#
        )?;
    }

    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    sol! {
        function test_round_trip(bytes calldata input, uint64 expected_choice);
    }

    // (encoded BCS bytes, expected variant index).
    let cases: Vec<(Vec<u8>, u64)> = vec![
        // index 0   — single-byte ULEB128
        (vec![0x00], 0),
        // index 5   — single-byte ULEB128, sparse
        (vec![0x05], 5),
        // index 128 — first two-byte ULEB128, sparse
        (vec![0x80, 0x01], 128),
        // index 300 — two-byte ULEB128 + u32 payload
        (
            {
                let mut v = vec![0xac, 0x02];
                v.extend_from_slice(&0xdead_beef_u32.to_le_bytes());
                v
            },
            300,
        ),
    ];

    for (input_bytes, expected_choice) in cases {
        let input = Bytes::copy_from_slice(&input_bytes);
        let fct_args = test_round_tripCall {
            input,
            expected_choice,
        };
        let fct_args = fct_args.abi_encode().into();
        test_contract(bytecode.clone(), fct_args);
    }

    Ok(())
}

/// Regression test for the trailing-comma bug in the complex-Enum codegen.
///
/// When an enum reaches the struct-backed `Enum` path with *no* payload-bearing
/// variants, the generated struct has only the `choice` field. The case
/// constructors and the deserializer return statement used to hardcode a comma
/// after `choice`, producing invalid Solidity like `Foo(uint64(0), )`.
///
/// Two shapes hit this path:
///   - sparse all-Unit enums (sparse indices disqualify SimpleEnum);
///   - contiguous all-Unit enums with >256 variants (over the Solidity-enum cap).
///
/// This test exercises the sparse case; the fix covers both.
#[test]
fn test_enum_sparse_all_unit() -> anyhow::Result<()> {
    use serde_reflection::{ContainerFormat, Named, Registry, VariantFormat};
    use std::collections::BTreeMap;

    let mut variants: BTreeMap<u32, Named<VariantFormat>> = BTreeMap::new();
    for &(idx, name) in &[(0u32, "Zero"), (5, "Five"), (128, "OneTwentyEight")] {
        variants.insert(
            idx,
            Named {
                name: name.into(),
                value: VariantFormat::Unit,
            },
        );
    }

    let mut registry = Registry::new();
    registry.insert("AllUnitSparse".into(), ContainerFormat::Enum(variants));

    let dir = tempdir().unwrap();
    let path = dir.path();

    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let config = CodeGeneratorConfig::new("Library".to_string());
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The bug manifests as `Foo(..., )` in the generated source; assert it
    // never appears so a future regression fails at the unit-test layer
    // before we even invoke solc.
    let generated = std::fs::read_to_string(&test_library_path)?;
    assert!(
        !generated.contains(", )"),
        "generator emitted a trailing comma in a struct constructor:\n{generated}"
    );

    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;
        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_round_trip(bytes calldata input, uint64 expected_choice) external {{
        Library.AllUnitSparse memory t = Library.bcs_deserialize_AllUnitSparse(input);
        require(t.choice == expected_choice, "wrong choice");
        bytes memory input_rev = Library.bcs_serialize_AllUnitSparse(t);
        require(input.length == input_rev.length, "length mismatch");
        for (uint256 i=0; i<input.length; i++) {{
            require(input[i] == input_rev[i], "byte mismatch");
        }}
    }}

}}
"#
        )?;
    }

    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    sol! {
        function test_round_trip(bytes calldata input, uint64 expected_choice);
    }

    let cases: Vec<(Vec<u8>, u64)> =
        vec![(vec![0x00], 0), (vec![0x05], 5), (vec![0x80, 0x01], 128)];

    for (input_bytes, expected_choice) in cases {
        let input = Bytes::copy_from_slice(&input_bytes);
        let fct_args = test_round_tripCall {
            input,
            expected_choice,
        };
        let fct_args = fct_args.abi_encode().into();
        test_contract(bytecode.clone(), fct_args);
    }

    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ComplexStruct {
    v1: [u8; 32],
    v2: [u8; 20],
    v3: [u16; 10],
}

#[test]
fn test_bytes32_and_related() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<ComplexStruct>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The library code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function get_bytes32() internal returns (bytes32) {{
        bytes memory vect;
        for (uint8 i=0; i<32; i++) {{
            vect = abi.encodePacked(vect, i);
        }}
        bytes32 dest;
        assembly {{
            dest := mload(add(vect, 0x20))
        }}
        return dest;
    }}

    function get_bytes20() internal returns (bytes20) {{
        bytes memory vect;
        for (uint8 i=0; i<20; i++) {{
            vect = abi.encodePacked(vect, i);
        }}
        bytes20 dest;
        assembly {{
            dest := mload(add(vect, 0x20))
        }}
        return dest;
    }}

    function test_deserialization(bytes calldata input) external {{
      Library.ComplexStruct memory t = Library.bcs_deserialize_ComplexStruct(input);

      bytes memory input_rev = Library.bcs_serialize_ComplexStruct(t);
      require(input.length == input_rev.length);
      for (uint256 i=0; i<input.length; i++) {{
        require(input[i] == input_rev[i]);
      }}
      require(t.v1 == get_bytes32());
      require(t.v2 == get_bytes20());
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    // Building the test entry
    let mut v1 = [0_u8; 32];
    for (i, item) in v1.iter_mut().enumerate() {
        *item = i as u8;
    }
    //
    let mut v2 = [0_u8; 20];
    for (i, item) in v2.iter_mut().enumerate() {
        *item = i as u8;
    }
    //
    let mut v3 = [0_u16; 10];
    for (i, item) in v3.iter_mut().enumerate() {
        *item = i as u16;
    }
    //
    let t = ComplexStruct { v1, v2, v3 };
    let expected_input = bcs::to_bytes(&t).unwrap();

    // Building the input to the smart contract
    sol! {
        function test_deserialization(bytes calldata input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode.clone(), fct_args);
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NestingBools {
    v1: Option<bool>,
    v2: Option<bool>,
    v3: Option<bool>,
    v4: bool,
}

#[test]
fn test_nesting_bools() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<NestingBools>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The library code
    let test_library_path = path.join("Library.sol");
    {
        let mut test_library_file = File::create(&test_library_path)?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // The test code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;

        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{

    function test_deserialization(bytes calldata input) external {{
      Library.NestingBools memory t = Library.bcs_deserialize_NestingBools(input);

      bytes memory input_rev = Library.bcs_serialize_NestingBools(t);
      require(input.length == input_rev.length);
      for (uint256 i=0; i<input.length; i++) {{
        require(input[i] == input_rev[i]);
      }}
    }}

}}
"#
        )?;
    }

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;
    //
    let t = NestingBools {
        v1: None,
        v2: Some(true),
        v3: Some(false),
        v4: true,
    };
    let expected_input = bcs::to_bytes(&t).unwrap();

    // Building the input to the smart contract
    sol! {
        function test_deserialization(bytes calldata input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode.clone(), fct_args);
    Ok(())
}
