#![allow(unused)]
use crate::{
	common,
	indent::{IndentConfig, IndentedWriter},
	CodeGeneratorConfig,
};
use heck::CamelCase;
use include_dir::include_dir as include_directory;
use indoc::{formatdoc, indoc, writedoc};
use serde_reflection::{ContainerFormat, Format, FormatHolder, Named, Registry, VariantFormat};
use std::{
	collections::{BTreeMap, HashMap},
	io::{Result, Write},
	path::PathBuf,
};

/// Main configuration object for code-generation in TypeScript, powered by
/// the Deno runtime.
pub struct CodeGenerator<'a> {
	/// Language-independent configuration.
	config: &'a CodeGeneratorConfig,
	/// Mapping from external type names to fully-qualified class names (e.g. "MyClass" -> "com.my_org.my_package.MyClass").
	/// Derived from `config.external_definitions`.
	external_qualified_names: HashMap<String, String>,
	/// vector of namespaces to import
	namespaces_to_import: Vec<String>,
}

/// Shared state for the code generation of a TypeScript source file.
struct TypeScriptEmitter<'a, T> {
	/// Writer.
	out: IndentedWriter<T>,
	/// Generator.
	generator: &'a CodeGenerator<'a>,
}

impl<'a> CodeGenerator<'a> {
	/// Create a TypeScript code generator for the given config.
	pub fn new(config: &'a CodeGeneratorConfig) -> Self {
		if config.c_style_enums {
			panic!("TypeScript does not support generating c-style enums");
		}
		let mut external_qualified_names = HashMap::new();
		for (namespace, names) in &config.external_definitions {
			for name in names {
				external_qualified_names.insert(
					name.to_string(),
					format!("{}.{}", namespace.to_camel_case(), name),
				);
			}
		}
		Self {
			config,
			external_qualified_names,
			namespaces_to_import: config.external_definitions.keys().map(|k| k.to_string()).collect::<Vec<_>>(),
		}
	}
	
	/// Output class definitions for `registry` in a single source file.
	pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result<()> {
		let mut emitter = TypeScriptEmitter {
			out: IndentedWriter::new(out, IndentConfig::Tab),
			generator: self,
		};
		
		emitter.output_preamble()?;
		
		for (name, format) in registry {
			writeln!(emitter.out)?;
			emitter.output_container_typedef(name, format)?;
		}
		for (name, format) in registry {
			writeln!(emitter.out)?;
			emitter.generate_container(name, format)?;
		}
		
		Ok(())
	}
}

impl<'a, T: Write> TypeScriptEmitter<'a, T> {
	fn output_preamble(&mut self) -> Result<()> {
		writeln!(self.out, r#"import type * as $t from "./serde""#)?;
		writeln!(self.out, r#"import {{ BincodeReader, BincodeWriter }} from "./bincode""#)?;
		for namespace in self.generator.namespaces_to_import.iter() {
			writeln!(self.out, "import * as {} from '../{}/mod';\n", namespace.to_camel_case(), namespace)?;
		}
		Ok(())
	}
	
	fn generate_container(&mut self, name: &str, container: &ContainerFormat) -> Result<()> {
		// ENCODE
		writeln!(self.out, "export const {name} = {{")?;
		self.out.indent();
		
		writeln!(self.out, "encode(value: {name}, writer = new BincodeWriter()) {{")?;
		self.out.indent();
		
		match container {
			ContainerFormat::UnitStruct => {
				writeln!(self.out, "{}", self.quote_write_value("null", &Format::Unit))?;
			}
			ContainerFormat::Struct(fields) => {
				for field in fields.iter() {
					writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}", field.name), &field.value))?;
				}
			}
			ContainerFormat::NewTypeStruct(inner_type) => {
				writeln!(self.out, "{}", self.quote_write_value(&format!("value"), inner_type))?;	
			}
			ContainerFormat::TupleStruct(inner_types) => {
				for (i, inner) in inner_types.iter().enumerate() {
					writeln!(self.out, "{}", self.quote_write_value(&format!("value[{i}]"), inner))?;	
				}
			}
			ContainerFormat::Enum(variants) => {
				self.generate_enum_container(name, variants)?;
				return Ok(());
			}
		}

		writeln!(self.out, "return writer.getBytes()")?;
		
		self.out.unindent();
		writeln!(self.out, "}},")?;
		

		// DECODE
		writeln!(self.out, "decode(input: Uint8Array, reader = new BincodeReader(input)) {{")?;
		self.out.indent();
				
		match container {
			ContainerFormat::UnitStruct => {
				writeln!(self.out, "const value: $t.unit = {}", self.quote_read_value(&Format::Unit))?;
			}
			ContainerFormat::NewTypeStruct(inner) => {
				writeln!(self.out, "const value: {name} = {}", self.quote_read_value(inner))?;
			}
			ContainerFormat::TupleStruct(inner_types) => {
				writeln!(self.out, "const value: {name} = {}", self.quote_read_value(&Format::Tuple(inner_types.clone())))?;
			}
			_ => { writeln!(self.out, "const value = {{}} as {name}")?; }
		}
		
		match container {
			ContainerFormat::UnitStruct => {  /* set at initialization */ }
			ContainerFormat::TupleStruct(inner_types) => { /* set at initialization */ }
			ContainerFormat::NewTypeStruct(inner_type) => { /* set at initialization */  }
			ContainerFormat::Struct(fields) => {
				for field in fields.iter() {
					writeln!(self.out, "value.{} = {}", field.name, self.quote_read_value(&field.value))?;
				}
			}
			ContainerFormat::Enum(..) => { /* handled before with generate_enum_container() */ }
		}

		writeln!(self.out, "return value")?;
		
		self.out.unindent(); 
		writeln!(self.out, "}}")?; // decode end
		
		self.out.unindent(); 
		writeln!(self.out, "}}")?; // object end		
		
		Ok(())
	}
	
	fn generate_enum_container(&mut self, name: &str, variants: &BTreeMap<u32, Named<VariantFormat>>) -> Result<()> {
		writeln!(self.out, "switch (value.$) {{")?;
		self.out.indent();
		
		for (index, variant) in variants {
			writeln!(self.out, r#"case "{}": {{"#, variant.name)?;
			self.out.indent();
			writeln!(self.out, "writer.writeVariantIndex({index})");
			
			match &variant.value {
				VariantFormat::Unit => {
					writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}", &variant.name), &Format::Unit));
				},
				VariantFormat::NewType(inner) => {
					writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}", &variant.name), inner));
				}
				VariantFormat::Tuple(members) => {
					let tuple = Format::Tuple(members.clone());
					writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}", &variant.name), &tuple));
				}
				VariantFormat::Struct(fields) => {
					for field in fields {
						writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}.{}", variant.name, field.name), &field.value))?;
					}
				}
				VariantFormat::Variable(_) => panic!("not supported")
			}
			writeln!(self.out, "break")?;
			self.out.unindent();
			writeln!(self.out, "}}")?; // case end
		}
		
		self.out.unindent();
		writeln!(self.out, "}}")?; // switch end
		
		writeln!(self.out, "return writer.getBytes()");
		self.out.unindent();
		writeln!(self.out, "}},")?; // encode end
		
		writeln!(self.out, "decode(input: Uint8Array, reader = new BincodeReader(input)) {{")?;
		self.out.indent();
		
		writeln!(self.out, r#"let value: {name}"#);

		writeln!(self.out, "switch (reader.readVariantIndex()) {{")?;
		self.out.indent();
		
		for (index, variant) in variants {
			writeln!(self.out, r#"case {index}: {{"#)?;
			self.out.indent();
			
			writeln!(self.out, r#"value = {{ $: "{}" }} as $t.WrapperOfCase<{}, "{}">"#, variant.name, name, variant.name);

			match &variant.value {
				VariantFormat::Unit => {
					writeln!(self.out, "value.{} = {}", variant.name, self.quote_read_value(&Format::Unit));
				},
				VariantFormat::Tuple(members) => {
					let tuple = Format::Tuple(members.clone());
					writeln!(self.out, "value.{} = {}", variant.name, self.quote_read_value(&tuple));
				}
				VariantFormat::NewType(inner) => {
					writeln!(self.out, "value.{} = {}", variant.name, self.quote_read_value(inner));
				}
				VariantFormat::Struct(fields) => {
					writeln!(self.out, r#"value.{var} = {{}} as $t.WrapperOfCase<{name}, "{var}">["{var}"]"#, var = variant.name);
					for field in fields {
						writeln!(self.out, "value.{}.{} = {}", variant.name, field.name, self.quote_read_value(&field.value))?;
					}
				}
				VariantFormat::Variable(_) => panic!("not supported")
			}

			writeln!(self.out, "break")?;
			self.out.unindent();
			writeln!(self.out, "}}")?; // case end
		}

		self.out.unindent(); 
		writeln!(self.out, "}}")?; // switch end

		writeln!(self.out)?;
		writeln!(self.out, "return value")?;
		
		self.out.unindent();
		writeln!(self.out, "}}")?; // decode end

		self.out.unindent();
		writeln!(self.out, "}}")?; // object end
		
		Ok(())
	}

	fn output_container_typedef(&mut self, name: &str, container: &ContainerFormat) -> Result<()> {
		match container {
			ContainerFormat::UnitStruct => {
				writeln!(self.out, "export type {name} = $t.unit")?;
			}
			ContainerFormat::TupleStruct(fields) => {
				writeln!(self.out, "export type {name} = [{}]", self.quote_types(&fields, ", "))?;
				self.out.unindent();
			}
			ContainerFormat::Struct(fields) => {
				writeln!(self.out, "export type {name} = {{")?;
				self.out.indent();
				for field in fields {
					match field.value {
						Format::Unit | Format::Option {..} => {
							writeln!(self.out, "{}?: {},", field.name, self.quote_type(&field.value))?;
						}
						_ => { writeln!(self.out, "{}: {},", field.name, self.quote_type(&field.value))?; }
					}
				}
				self.out.unindent();
				writeln!(self.out, "}}")?;
			}
			ContainerFormat::NewTypeStruct(format) => {
				writeln!(self.out, "export type {name} = {}", self.quote_type(format))?;
			}
			ContainerFormat::Enum(variants) => { 
				// TODO https://github.com/zefchain/serde-reflection/issues/45
				writeln!(self.out, "export type {name} = ")?;
				self.out.indent();
				for (_index, variant) in variants {
					match &variant.value {
						VariantFormat::Unit => {
							writeln!(self.out, r#"| {{ $: "{0}", {0}?: {1} }}"#, variant.name, self.quote_type(&Format::Unit))?;
						}
						VariantFormat::Struct(fields) => {
							let fields_str = fields.iter().map(|f| format!("{}: {}", f.name, self.quote_type(&f.value))).collect::<Vec<_>>().join(", ");
							writeln!(self.out, r#"| {{ $: "{0}", {0}: {{ {1} }} }}"#, variant.name, fields_str)?;
						}
						VariantFormat::NewType(t) => {
							writeln!(self.out, r#"| {{ $: "{0}", {0}: {1} }}"#, variant.name, self.quote_type(&t))?;
						}
						VariantFormat::Tuple(t) => {
							writeln!(self.out, r#"| {{ $: "{0}", {0}: {1} }}"#, variant.name, self.quote_type(&Format::Tuple(t.clone())))?;
						}
						VariantFormat::Variable(v) => panic!("unknown variant format")
					}
				}
				self.out.unindent();
			}
			_ => panic!("format not implemented")
		}
		
		Ok(())
	}
	
	fn quote_qualified_name(&self, name: &str) -> String {
		self.generator.external_qualified_names.get(name).cloned().unwrap_or_else(|| name.to_string())
	}
	
	fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
		let path = vec![name.to_string()];
		if let Some(doc) = self.generator.config.comments.get(&path) {
			let text = textwrap::indent(doc, " * ").replace("\n\n", "\n *\n");
			writeln!(self.out, "/**\n{} */", text)?;
		}
		Ok(())
	}
	
	fn quote_type(&self, format: &Format) -> String {
		use Format::*;
		let str = match format {
			Unit  => "$t.unit",
			Bool  => "$t.bool",
			I8    => "$t.i8",
			I16   => "$t.i16",
			I32   => "$t.i32",
			I64   => "$t.i64",
			I128  => "$t.i128",
			U8    => "$t.u8",
			U16   => "$t.u16",
			U32   => "$t.u32",
			U64   => "$t.u64",
			U128  => "$t.u128",
			F32   => "$t.f32",
			F64   => "$t.f64",
			Char  => "$t.char",
			Str   => "$t.str",
			Bytes => "$t.bytes",
			
			Option(format)                       => &format!("$t.Optional<{}>", self.quote_type(format)),
			Seq(format)                          => &format!("$t.Seq<{}>", self.quote_type(format)),
			Map { key, value }                   => &format!("$t.Map<{}, {}>", self.quote_type(key), self.quote_type(value)),
			Tuple(formats)                       => &format!("$t.Tuple<[{}]>", self.quote_types(formats, ", ")),
			TupleArray { content, .. }           => &format!("$t.ListTuple<[{}]>", self.quote_type(content)),
			
			TypeName(x) => &self.quote_qualified_name(x),
			
			Variable(_) => panic!("unexpected value"),
		};
		str.to_string()
	}
	
	fn quote_types(&self, formats: &[Format], sep: &str) -> String {
		formats.iter().map(|f| self.quote_type(f)).collect::<Vec<_>>().join(sep)
	}
	
	fn quote_write_value(&self, value: &str, format: &Format) -> String {
		use Format::*;
		match format {
			TypeName(typename) => format!("{typename}.encode({value}, writer)"),
			Unit        => format!("writer.writeUnit({value})"),
			Bool        => format!("writer.writeBool({value})"),
			I8          => format!("writer.writeI8({value})"),
			I16         => format!("writer.writeI16({value})"),
			I32         => format!("writer.writeI32({value})"),
			I64         => format!("writer.writeI64({value})"),
			I128        => format!("writer.writeI128({value})"),
			U8          => format!("writer.writeU8({value})"),
			U16         => format!("writer.writeU16({value})"),
			U32         => format!("writer.writeU32({value})"),
			U64         => format!("writer.writeU64({value})"),
			U128        => format!("writer.writeU128({value})"),
			F32         => format!("writer.writeF32({value})"),
			F64         => format!("writer.writeF64({value})"),
			Char        => format!("writer.writeChar({value})"),
			Str         => format!("writer.writeString({value})"),
			Bytes       => format!("writer.writeBytes({value})"),
			Option(inner) => {
				formatdoc! {
					"
						if ({value}) {{
							writer.writeOptionTag(true)
							{}
						}} 
						else writer.writeOptionTag(false)
                    ",
					self.quote_write_value(value, inner)
				}
			},
			Seq(format) => {
				formatdoc!("
					writer.writeLength({value}.length)
					for (const item of {value}) {{
						{}
					}}", 
					self.quote_write_value("item", format)
				)
			}
			Map { key: map_key, value: map_value } => {
				format! {
					"writer.writeMap({value}, {}, {})",
					self.quote_write_value("", map_key).replace("()", ".bind(writer)"),
					self.quote_write_value("", map_value).replace("()", ".bind(writer)")
				}
			}
			Tuple(formats) => {
				use std::fmt::Write;
				let mut lines = Vec::new();
				for (index, format) in formats.iter().enumerate() {
					let expr = format!("{value}[{}]", index);
					lines.push(self.quote_write_value(&expr, format));
				}
				lines.join("\n")
			}
			TupleArray { content, .. } => {
				formatdoc!("
					for (const item of {value}) {{
						{}
					}}",
					self.quote_write_value("item[0]", content)
				)
			}
			_ => panic!("unexpected case"),
		}
	}
	
	fn quote_read_value(&self, format: &Format) -> String {
		use Format::*;
		let str = match format {
			TypeName(name) => &format!("{}.decode(input, reader)", self.quote_qualified_name(name)),
			Unit  => "reader.readUnit()",
			Bool  => "reader.readBool()",
			I8    => "reader.readI8()",
			I16   => "reader.readI16()",
			I32   => "reader.readI32()",
			I64   => "reader.readI64()",
			I128  => "reader.readI128()",
			U8    => "reader.readU8()",
			U16   => "reader.readU16()",
			U32   => "reader.readU32()",
			U64   => "reader.readU64()",
			U128  => "reader.readU128()",
			F32   => "reader.readF32()",
			F64   => "reader.readF64()",
			Char  => "reader.readChar()",
			Str   => "reader.readString()",
			Bytes => "reader.readBytes()",
			Option(format) => {
				&format!("reader.readOptionTag() ? {} : null", self.quote_read_value(format))
			}
			Seq(format) => {
				&format!(
					"reader.readList<{}>(() => {})",
					self.quote_type(format),
					self.quote_read_value(format)
				)
			}
			Map { key, value } => {
				&format!(
					"reader.readMap<{}, {}>({}, {})",
					self.quote_type(key),
					self.quote_type(value),
					self.quote_read_value(key).replace("()", ".bind(reader)"),
					self.quote_read_value(value).replace("()", ".bind(reader)"),
				)
			}
			Tuple(formats) => {
				&format!(
					"[{}]",formats.iter()
					.map(|f| format!("{}", self.quote_read_value(f)))
					.collect::<Vec<_>>()
					.join(", ")
				)
			}
			TupleArray { content, size } => {
				&format!(
					"reader.readList<{}>(() => {}, {})",
					self.quote_type(format), self.quote_read_value(content), size,
				)
			}
			Variable(_) => panic!("unsupported value")
		};
		str.to_string()
	}
	
}