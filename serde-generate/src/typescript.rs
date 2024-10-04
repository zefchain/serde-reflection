#![allow(unused)]
use crate::{
	common,
	indent::{IndentConfig, IndentedWriter},
	CodeGeneratorConfig,
};
use heck::{CamelCase, SnakeCase};
use include_dir::include_dir as include_directory;
use indoc::{formatdoc, indoc, writedoc};
use serde_reflection::{ContainerFormat, Format, FormatHolder, Named, Registry, VariantFormat};
use std::{
	collections::{BTreeMap, HashMap},
	io::{Result, Write},
	path::PathBuf,
};

/// Main configuration object for code-generation in TypeScript
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
			emitter.generate_container_typedef(name, format)?;
		}
		if self.config.serialization {
			for (name, format) in registry {
				writeln!(emitter.out)?;
				emitter.generate_container(name, format)?;
			}
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
		// Encode
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
					writeln!(self.out, "{}", self.quote_write_value(&format!("value.${i}"), inner))?;	
				}
			}
			ContainerFormat::Enum(variants) => {
				self.generate_enum_container(name, variants)?;
				return Ok(());
			}
		}

		writeln!(self.out, "return writer.get_bytes()")?;
		
		self.out.unindent();
		writeln!(self.out, "}},")?;
		

		// Decode
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
				writeln!(self.out, "const value: {name} = {{")?;
				self.out.indent();
				for (i, inner) in inner_types.iter().enumerate() {
					writeln!(self.out, "${i}: {},", self.quote_read_value(&inner))?;	
				}
				self.out.unindent();
				writeln!(self.out, "}}")?;
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
			writeln!(self.out, r#"case "{}": {{"#, variant.name.to_snake_case())?;
			self.out.indent();
			writeln!(self.out, "writer.write_variant_index({index})");
			
			match &variant.value {
				VariantFormat::Unit => {
					writeln!(self.out, "{}", self.quote_write_value("value.$0", &Format::Unit));
				},
				VariantFormat::NewType(inner) => {
					writeln!(self.out, "{}", self.quote_write_value("value.$0", inner));
				}
				VariantFormat::Tuple(members) => {
					let tuple = Format::Tuple(members.clone());
					writeln!(self.out, "{}", self.quote_write_value("value", &tuple));
				}
				VariantFormat::Struct(fields) => {
					for field in fields {
						writeln!(self.out, "{}", self.quote_write_value(&format!("value.{}", field.name), &field.value))?;
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
		
		writeln!(self.out, "return writer.get_bytes()");
		self.out.unindent();
		writeln!(self.out, "}},")?; // encode end
		
		writeln!(self.out, "decode(input: Uint8Array, reader = new BincodeReader(input)) {{")?;
		self.out.indent();
		
		writeln!(self.out, r#"let value: {name}"#);

		writeln!(self.out, "switch (reader.read_variant_index()) {{")?;
		self.out.indent();
		
		for (index, variant) in variants {
			writeln!(self.out, r#"case {index}: {{"#)?;
			self.out.indent();
			
			writeln!(self.out, r#"value = {{"#);
			self.out.indent();
			writeln!(self.out, r#"$: "{0}","#, variant.name.to_snake_case());

			match &variant.value {
				VariantFormat::Unit => {
					writeln!(self.out, "$0: {}", self.quote_read_value(&Format::Unit));
				},
				VariantFormat::Tuple(members) => {
					let tuple = Format::Tuple(members.clone());
					for (i, member) in members.iter().enumerate() {
						writeln!(self.out, "${i}: {},", self.quote_read_value(&member));
					}
				}
				VariantFormat::NewType(inner) => {
					writeln!(self.out, "$0: {},", self.quote_read_value(inner));
				}
				VariantFormat::Struct(fields) => {
					for field in fields {
						writeln!(self.out, "{}: {},", field.name, self.quote_read_value(&field.value))?;
					}
				}
				VariantFormat::Variable(_) => panic!("not supported")
			}

			self.out.unindent();
			writeln!(self.out, r#"}} satisfies Extract<{0}, {{ $: "{1}" }}>"#, name, variant.name.to_snake_case())?;

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

	fn generate_container_typedef(&mut self, name: &str, container: &ContainerFormat) -> Result<()> {
		match container {
			ContainerFormat::UnitStruct => {
				writeln!(self.out, "export type {name} = $t.unit")?;
			}
			ContainerFormat::TupleStruct(fields) => {
				writeln!(self.out, "export type {name} = $t.Tuple<[{}]>", self.quote_types(&fields, ", "))?;
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
					let variant_name_snake = variant.name.to_snake_case();
					match &variant.value {
						VariantFormat::Unit => {
							writeln!(self.out, r#"| {{ $: "{0}", $0?: {1} }}"#, variant_name_snake, self.quote_type(&Format::Unit))?;
						}
						VariantFormat::Struct(fields) => {
							let fields_str = fields.iter().map(|f| format!("{}: {}", f.name, self.quote_type(&f.value))).collect::<Vec<_>>().join(", ");
							writeln!(self.out, r#"| {{ $: "{0}", {1} }}"#, variant_name_snake, fields_str)?;
						}
						VariantFormat::NewType(t) => {
							writeln!(self.out, r#"| {{ $: "{0}", $0: {1} }}"#, variant_name_snake, self.quote_type(&t))?;
						}
						VariantFormat::Tuple(t) => {
							writeln!(self.out, r#"| {{ $: "{0}" }} & {1}"#, variant_name_snake, self.quote_type(&Format::Tuple(t.clone())))?;
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
			Unit        => format!("writer.write_unit({value})"),
			Bool        => format!("writer.write_bool({value})"),
			I8          => format!("writer.write_i8({value})"),
			I16         => format!("writer.write_i16({value})"),
			I32         => format!("writer.write_i32({value})"),
			I64         => format!("writer.write_i64({value})"),
			I128        => format!("writer.write_i128({value})"),
			U8          => format!("writer.write_u8({value})"),
			U16         => format!("writer.write_u16({value})"),
			U32         => format!("writer.write_u32({value})"),
			U64         => format!("writer.write_u64({value})"),
			U128        => format!("writer.write_u128({value})"),
			F32         => format!("writer.write_f32({value})"),
			F64         => format!("writer.write_f64({value})"),
			Char        => format!("writer.write_char({value})"),
			Str         => format!("writer.write_string({value})"),
			Bytes       => format!("writer.write_bytes({value})"),			Option(inner) => {
				formatdoc! {
					"
						if ({value}) {{
							writer.write_option_tag(true)
							{}
						}} 
						else writer.write_option_tag(false)
                    ",
					self.quote_write_value(value, inner)
				}
			},
			Seq(format) => {
				formatdoc!("
					writer.write_length({value}.length)
					for (const item of {value}) {{
						{}
					}}", 
					self.quote_write_value("item", format)
				)
			}
			Map { key: map_key, value: map_value } => {
				format! {
					"writer.write_map({value}, {}, {})",
					self.quote_write_value("", map_key).replace("()", ".bind(writer)"),
					self.quote_write_value("", map_value).replace("()", ".bind(writer)")
				}
			}
			Tuple(formats) => {
				use std::fmt::Write;
				let mut lines = Vec::new();
				for (index, format) in formats.iter().enumerate() {
					let expr = format!("{value}.${}", index);
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
			Unit  => "reader.read_unit()",
			Bool  => "reader.read_bool()",
			I8    => "reader.read_i8()",
			I16   => "reader.read_i16()",
			I32   => "reader.read_i32()",
			I64   => "reader.read_i64()",
			I128  => "reader.read_i128()",
			U8    => "reader.read_u8()",
			U16   => "reader.read_u16()",
			U32   => "reader.read_u32()",
			U64   => "reader.read_u64()",
			U128  => "reader.read_u128()",
			F32   => "reader.read_f32()",
			F64   => "reader.read_f64()",
			Char  => "reader.read_char()",
			Str   => "reader.read_string()",
			Bytes => "reader.read_bytes()",	
			Option(format) => {
				&format!("reader.read_option_tag() ? {} : null", self.quote_read_value(format))
			}
			Seq(format) => {
				&format!(
					"reader.read_list<{}>(() => {})",
					self.quote_type(format),
					self.quote_read_value(format)
				)
			}
			Map { key, value } => {
				&format!(
					"reader.read_map<{}, {}>({}, {})",
					self.quote_type(key),
					self.quote_type(value),
					self.quote_read_value(key).replace("()", ".bind(reader)"),
					self.quote_read_value(value).replace("()", ".bind(reader)"),
				)
			}
			Tuple(formats) => {
				let mut buf = Vec::new();
				let mut writer = IndentedWriter::new(&mut buf, IndentConfig::Tab);
				writeln!(writer, "{{");
				writer.indent();
				for (i, f) in formats.iter().enumerate() {
					writeln!(writer, "${i}: {},", self.quote_read_value(f));
				}
				writer.unindent();
				write!(writer, "}}");
				Box::leak(String::from_utf8(buf).unwrap().into_boxed_str())
			}
			TupleArray { content, size } => {
				&format!(
					"reader.read_list<{}>(() => {}, {})",
					self.quote_type(format), self.quote_read_value(content), size,
				)
			}
			Variable(_) => panic!("unsupported value")
		};
		str.to_string()
	}
	
}