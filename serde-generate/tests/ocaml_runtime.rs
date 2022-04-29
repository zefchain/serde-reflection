use serde_generate::{
    ocaml, test_utils, test_utils::Runtime, CodeGeneratorConfig, SourceInstaller,
};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "\"{}\"",
        bytes
            .iter()
            .map(|x| format!("\\{:03}", x))
            .collect::<Vec<_>>()
            .join("")
    )
}

#[test]
fn test_ocaml_bcs_runtime() {
    test_ocaml_runtime(Runtime::Bcs);
}

#[test]
fn test_ocaml_bincode_runtime() {
    test_ocaml_runtime(Runtime::Bincode);
}

fn test_ocaml_runtime(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir0 = tempdir().unwrap();
    let dir = dir0.path();
    let installer = ocaml::Installer::new(dir.to_path_buf());
    let runtime_str = match runtime {
        Runtime::Bcs => {
            installer.install_bcs_runtime().unwrap();
            "bcs"
        }
        Runtime::Bincode => {
            installer.install_bincode_runtime().unwrap();
            "bincode"
        }
    };

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);

    let dir_path = dir.join(&config.module_name());
    std::fs::create_dir_all(&dir_path).unwrap();

    let dune_project_source_path = dir.join("dune-project");
    let mut dune_project_file = std::fs::File::create(dune_project_source_path).unwrap();
    writeln!(dune_project_file, "(lang dune 3.0)").unwrap();

    let dune_source_path = dir_path.join("dune");
    let mut dune_file = std::fs::File::create(dune_source_path).unwrap();

    writeln!(
        dune_file,
        r#"
(env (_ (flags (:standard -w -30-42))))

(executable
 (name test)
 (modules test)
 (preprocess (pps ppx)) 
 (libraries {}_runtime))
"#,
        runtime_str
    )
    .unwrap();

    let source_path = dir_path.join("test.ml");
    println!("{:?}", source_path);
    let mut source = File::create(&source_path).unwrap();
    let generator = ocaml::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let positive_encodings: Vec<_> = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    let negative_encodings: Vec<_> = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    writeln!(
        source,
        r#"
let () = 
  List.iter (fun s ->
      let buffer = Bytes.of_string s in
      let sd = serde_data_de {{Serde.Deserialize.buffer; offset=0}} in
      let buffer2 = serde_data_ser sd in
      assert (buffer = buffer2)) [{}];

  List.iter (fun s ->
      let buffer = Bytes.of_string s in
      try 
        let _ = serde_data_de {{Serde.Deserialize.buffer; offset=0}} in
        ()
      with _ -> ()) [{}]
"#,
        positive_encodings.join("; "),
        negative_encodings.join("; ")
    )
    .unwrap();

    let status = Command::new("dune")
        .arg("exec")
        .arg("testing/test.exe")
        .arg("--root")
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());
}
