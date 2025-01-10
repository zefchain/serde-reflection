use crate::solidity_generation::get_bytecode;
use alloy_sol_types::sol;
use serde_generate::{solidity, CodeGeneratorConfig};
use std::{fs::File, io::Write};
use serde_reflection::Samples;
use tempfile::tempdir;
use serde::{Deserialize, Serialize};
use serde_reflection::{Tracer, TracerConfig};
use alloy_sol_types::SolValue as _;
use revm::db::InMemoryDB;
use revm::{
    primitives::{Address, ExecutionResult, TxKind, Output, Bytes},
    Evm,
};


fn test_contract_instantiation(bytecode: Bytes, encoded_args: Vec<u8>) {
    let mut vec: Vec<u8> = bytecode.0.to_vec();
    vec.extend_from_slice(&encoded_args);
    let tx_data = Bytes::copy_from_slice(&vec);

    let database = InMemoryDB::default();
    let address1 = Address::ZERO;
    let mut evm : Evm<'_, (), _> = Evm::builder()
        .with_ref_db(database)
        .modify_tx_env(|tx| {
            tx.clear();
            tx.caller = address1;
            tx.transact_to = TxKind::Create;
            tx.data = tx_data;
        })
        .build();

    let result : ExecutionResult = evm.transact_commit().unwrap();

    let ExecutionResult::Success { reason: _, gas_used: _, gas_refunded: _, logs: _, output } = result else {
        panic!("The execution failed to be done");
    };
    let Output::Create(_, address) = output else {
        panic!("Failure to create the contract");
    };
    assert!(address.is_some());
}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TestVec {
    pub vec: Vec<u32>,
}




fn test_vector_serialization_len(len: usize) -> anyhow::Result<()> {
    // Indexing the types
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<TestVec>(&samples).expect("a tracer entry");
    let registry = tracer.registry().expect("A registry");

    // The directories
    let dir = tempdir().unwrap();
    let path = dir.path();

    // The generated code
    let generated_code_path = path.join("generated_code.sol");
    let mut generated_code_file = File::create(&generated_code_path)?;
    let name = "generated_test".to_string();
    let config = CodeGeneratorConfig::new(name);
    let generator = solidity::CodeGenerator::new(&config);
    generator.output(&mut generated_code_file, &registry).unwrap();

    // The code for testing whether
    let test_code_path = path.join("test_code.sol");
    let mut source = File::create(&test_code_path)?;
    writeln!(
        source,
        r#"
include generated_code from "./generated_code.sol";

contract ExampleCode {{

    constructor(bytes memory input) {{
       TestVec t = bcs_deserialize_TestVec(input);
       require(t.vec.length == {len}, "The length is incorrect");
       require(t.vec[0] == 42, "incorrect value");
       require(t.vec[1] == 5, "incorrect value");
       require(t.vec[2] == 360, "incorrect value");
       require(t.vec[{len} - 1] == 0, "incorrect value");
    }}

}}

"#
    )?;

    // Compiling the code and reading it.
    let bytecode = get_bytecode(path, "test_code.sol")?;


    // Building the test entry
    let mut vec = vec![0 as u32; len];
    vec[0] = 42;
    vec[1] = 5;
    vec[2] = 360;
    let t = TestVec { vec };
    let expected_input = bcs::to_bytes(&t).expect("Failed serialization");

    // Building the input to the smart contract
    sol! {
        struct ConstructorArgs {
            bytes input;
        }
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let args = ConstructorArgs { input };
    let encoded_args : Vec<u8> = args.abi_encode();

    test_contract_instantiation(bytecode, encoded_args);
    Ok(())
}



#[test]
fn test_vector_serialization() {
    for len in [30, 130] {
        test_vector_serialization_len(len).expect("successful run");
    }
}



