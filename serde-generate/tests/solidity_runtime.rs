use crate::solidity_generation::get_bytecode;
use alloy_sol_types::sol;
use serde_generate::{solidity, CodeGeneratorConfig};
use std::{fs::File, io::Write};
use serde_reflection::Samples;
use tempfile::tempdir;
use serde::{Deserialize, Serialize};
use serde_reflection::{Tracer, TracerConfig};
use alloy_sol_types::SolCall as _;
use revm::db::InMemoryDB;
use revm::{
    primitives::{Address, ExecutionResult, TxKind, Output, Bytes, U256},
    Evm,
};


fn test_contract_instantiation(bytecode: Bytes, encoded_args: Bytes) {
    let mut database = InMemoryDB::default();
    let gas_limit = 10000000_u64;
    
    let address1 = Address::ZERO;
    let contract_address = {
        let mut evm : Evm<'_, (), _> = Evm::builder()
            .with_ref_db(&mut database)
            .modify_tx_env(|tx| {
                tx.clear();
                tx.caller = address1;
                tx.transact_to = TxKind::Create;
                tx.gas_limit = gas_limit;
                tx.data = bytecode;
            })
            .build();

        let result : ExecutionResult = evm.transact_commit().unwrap();

        println!("result={:?}", result);
        let ExecutionResult::Success { reason: _, gas_used: _, gas_refunded: _, logs: _, output } = result else {
            panic!("The TxKind::Create execution failed to be done");
        };
        let Output::Create(_, Some(contract_address)) = output else {
            panic!("Failure to create the contract");
        };
        contract_address
    };

    let mut evm : Evm<'_, (), _> = Evm::builder()
        .with_ref_db(&mut database)
        .modify_tx_env(|tx| {
            tx.caller = address1;
            tx.transact_to = TxKind::Call(contract_address);
            tx.value = U256::from(0);
            tx.gas_limit = gas_limit;
            tx.data = encoded_args;
        })
        .build();

    let result : ExecutionResult = evm.transact_commit().unwrap();

    let ExecutionResult::Success { reason: _, gas_used: _, gas_refunded: _, logs: _, output } = result else {
        panic!("The TxKind::Call execution failed to be done");
    };

}


#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TestVec {
    pub vec: Vec<u32>,
}




fn test_vector_serialization_len(len: usize) -> anyhow::Result<()> {
    use crate::solidity_generation::print_file_content;
    println!("test_vector_serialization_len, step 1");
    // Indexing the types
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<TestVec>(&samples).expect("a tracer entry");
    let registry = tracer.registry().expect("A registry");
    println!("test_vector_serialization_len, step 2");

    // The directories
    let dir = tempdir().unwrap();
    let path = dir.path();
    println!("path={}", path.display());

    // The generated code
    let test_code_path = path.join("test_code.sol");
    {
        let mut test_code_file = File::create(&test_code_path)?;
        println!("test_vector_serialization_len, step 3");
        let name = "ExampleCodeBase".to_string();
        let config = CodeGeneratorConfig::new(name);
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_code_file, &registry).unwrap();
        println!("test_vector_serialization_len, step 4");

        writeln!(
            test_code_file,
            r#"

contract ExampleCode is ExampleCodeBase {{

    constructor() {{
    }}

    function test_deserialization(bytes calldata input) external {{
       TestVec memory t = bcs_deserialize_TestVec(input);
       require(t.vec.length == {len}, "The length is incorrect");
       require(t.vec[0] == 42, "incorrect value");
       require(t.vec[1] == 5, "incorrect value");
       require(t.vec[2] == 360, "incorrect value");
       require(t.vec[{len} - 1] == 0, "incorrect value");
    }}



}}

"#
        )?;

    }
    print_file_content(&test_code_path);


    // Compiling the code and reading it.
    println!("test_vector_serialization_len, step 6");
    let bytecode = get_bytecode(path, "test_code.sol", "ExampleCode")?;
    println!("bytecode={}", bytecode);


    // Building the test entry
    let mut vec = vec![0 as u32; len];
    vec[0] = 42;
    vec[1] = 5;
    vec[2] = 360;
    let t = TestVec { vec };
    let expected_input = bcs::to_bytes(&t).expect("Failed serialization");
    println!("expected_input={:?}", expected_input);
    
    // Building the input to the smart contract
    sol! {
      function test_deserialization(bytes memory input);
    }
    let input = Bytes::copy_from_slice(&expected_input);
    let fct_args = test_deserializationCall { input };
    let fct_args = fct_args.abi_encode().into();
    println!("fct_args={}", fct_args);


    test_contract_instantiation(bytecode, fct_args);
    Ok(())
}



#[test]
fn test_vector_serialization() {
    for len in [30, 130] {
        test_vector_serialization_len(len).expect("successful run");
    }
}



