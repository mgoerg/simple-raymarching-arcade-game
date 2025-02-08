// wgsl_utils.rs
// https://elyshaffir.github.io/Taiga-Blog/2022/01/08/using_include_statements_in_wgsl.html
use std::{fs, io, io::Read, path};

pub fn load_shader(name: &str) -> Result<wgpu::ShaderModuleDescriptor, io::Error> {
	let shader_code = load_shader_module(name)?;

	Ok(wgpu::ShaderModuleDescriptor {
		label: Some(name),
		source: wgpu::ShaderSource::Wgsl(shader_code.into()),
	})
}

fn load_shader_module(name: &str) -> Result<String, io::Error> {
	let base_path = path::PathBuf::from("src");
	let module_path = base_path.join(name).with_extension("wgsl");
	if !module_path.is_file() {
		panic!("Shader not found: {:?}", module_path);
	}

	let mut module_source = String::new();
	io::BufReader::new(fs::File::open(&module_path)?).read_to_string(&mut module_source)?;
	let mut module_string = String::new();

	let first_line = module_source.lines().next().unwrap();
	if first_line.starts_with("//!include") {
		for include in first_line.split_whitespace().skip(1) {
			module_string.push_str(&*load_shader_module(include).unwrap());
		}
	}

	module_string.push_str(&module_source);
	Ok(module_string)
}
