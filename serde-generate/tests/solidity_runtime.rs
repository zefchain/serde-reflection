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

// Exercise the LEB128 length-prefix encoder/decoder at each byte-count boundary.
// Single-byte LEB128 covers len < 128 (fast path). Two-byte covers 128..16384.
// Three-byte covers 16384..2**21.
#[test]
fn test_varint_length_boundaries() {
    for len in [1_usize, 127, 128, 129, 16383, 16384, 16385] {
        let mut vec = vec![0_u8; len];
        vec[0] = 42;
        let t = TestVec { vec };
        test_vector_serialization(t).unwrap();
    }
}

// Drive `bcs_serialize_len` / `bcs_deserialize_offset_len` directly with
// values that force the unchecked count and emit loops through every
// LEB128 byte-count from 1 up to the full uint256 maximum (37 bytes).
// Building Vec<u8> payloads large enough to hit 4+ byte LEB128 lengths
// would be impractical, so the test bypasses the TestVec wrapper and
// calls the library helpers directly from a thin Solidity harness.
#[test]
fn test_varint_unchecked_loop_coverage() -> anyhow::Result<()> {
    let registry = get_registry_from_type::<TestVec<u8>>();
    let dir = tempdir().unwrap();
    let path = dir.path();

    {
        let mut test_library_file = File::create(path.join("Library.sol"))?;
        let name = "Library".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_library_file, &registry).unwrap();
    }

    // (Solidity literal, expected encoded byte count).
    // Each line straddles a 7-bit byte boundary, so the count loop and
    // the emit loop together cover every iteration depth up to 37.
    let cases: &[(&str, usize)] = &[
        ("0", 1),
        ("127", 1),
        ("128", 2),
        ("16383", 2),
        ("16384", 3),
        ("2097151", 3),
        ("2097152", 4),
        ("268435455", 4),
        ("268435456", 5),
        ("34359738367", 5),
        ("34359738368", 6),
        ("4398046511103", 6),
        ("4398046511104", 7),
        ("562949953421311", 7),
        ("562949953421312", 8),
        ("72057594037927935", 8),
        ("72057594037927936", 9),
        ("9223372036854775807", 9),
        ("9223372036854775808", 10),
        ("type(uint256).max", 37),
    ];

    let mut asserts = String::new();
    for (literal, expected_len) in cases {
        use std::fmt::Write as _;
        writeln!(asserts, "        _check({literal}, {expected_len});")?;
    }

    {
        let mut test_code_file = File::create(path.join("test_code.sol"))?;
        writeln!(
            test_code_file,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "./Library.sol";

contract ExampleCode {{
    function _check(uint256 x, uint256 expected_len) internal pure {{
        bytes memory enc = Library.bcs_serialize_len(x);
        require(enc.length == expected_len, "byte count mismatch");
        (uint256 new_pos, uint256 decoded) = Library.bcs_deserialize_offset_len(0, enc);
        require(new_pos == enc.length, "new_pos mismatch");
        require(decoded == x, "round-trip value mismatch");
    }}

    function test_deserialization(bytes calldata) external pure {{
{asserts}    }}
}}
"#
        )?;
    }

    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;

    sol! {
        function test_deserialization(bytes calldata input);
    }
    let fct_args = test_deserializationCall {
        input: Bytes::new(),
    };
    let fct_args = fct_args.abi_encode().into();

    test_contract(bytecode, fct_args);
    Ok(())
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
