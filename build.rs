use std::{
    collections::{
        HashMap,
        HashSet,
    },
    fs,
    io::{self, BufReader, Error, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

// global set of source files, used for rerun-if-changed in build.rs 
// collects all files whose modification times are considered for the build
static mut SOURCE_FILES: Option<HashSet<PathBuf>> = None;

fn main() {
    println!("cargo::warning=build.rs running!");
    unsafe {
        SOURCE_FILES = Some(HashSet::new());
    }

    build_shaders().expect("Failed to build shaders");

    let source_files = unsafe {
        SOURCE_FILES
            .as_ref()
            .expect("SOURCE_FILES not initialized")
    };

    for file in source_files.iter() {
        println!("cargo:rerun-if-changed={}", file.to_string_lossy());
    }
    println!("cargo::warning=build.rs done!");
}

fn build_shaders() -> io::Result<()> {
    println!("cargo::warning=Building shaders.");
    let tasks: Vec<(&str, &str, Vec<&str>, fn(&Path, &[PathBuf], &Path) -> io::Result<()>)> = vec![(
        "src/shaders/shaderbuild/main_scene.wgsl",
        "src/shaders/shadersource/main_scene.wgsl",
        vec![
            "src/shaders/shadersource/*",
            "src/shaders/shadersource/common/*",
        ],
        preprocess_shader_file,
    )];

    run_tasks(tasks)
}



/// Recursively collects all files under `dir`, appending them to `out`.
fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Expand source paths, replacing any path ending in `'*'`
/// with all files under that directory (recursively).
fn expand_sources(sources: &[&Path]) -> io::Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for p in sources {
        // We need to check if the last character is `*`.
        // Path doesn't have an "ends_with('*')" method, so convert to string lossily.
        let p_str = p.to_string_lossy();
        if p_str.ends_with('*') {
            // Remove the trailing '*'
            let trimmed = p_str.trim_end_matches('*').trim_end_matches(std::path::MAIN_SEPARATOR);
            let dir_path = Path::new(trimmed);

            let mut add_files = Vec::new();
            collect_files_recursive(dir_path, &mut add_files)?;
            expanded.extend(add_files);
        } else {
            expanded.push(p.to_path_buf());
        }
    }

    Ok(expanded)
}

/// Check if `target` is older than any `sources`; if so, run `transform`.
fn build_if_out_of_date<F>(target: &Path, primary_source: &Path, sources: &[&Path], transform: F) -> io::Result<()>
where
    F: FnOnce(&Path, &[PathBuf], &Path) -> io::Result<()>,
{
    
    // Expand any wildcards:
    let expanded_sources = expand_sources(sources)?;

    let all_source_files = unsafe { &mut SOURCE_FILES }.as_mut().expect("SOURCE_FILES not initialized");
    all_source_files.extend(expanded_sources.iter().cloned());

    // Get modification time of the target; if missing, use the epoch so we definitely trigger a build.
    let target_mtime = fs::metadata(target)
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    // Check if any source file is strictly newer than the target.
    let mut rebuild_needed = false;
    // loop over expanded_sources and primary_source
    let primary_source_buffer = primary_source.to_path_buf();
    let sources_iter = expanded_sources.iter().chain(core::iter::once(&primary_source_buffer));
    for src in sources_iter {
        let src_mtime = fs::metadata(src)
            .and_then(|meta| meta.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if src_mtime > target_mtime {
            rebuild_needed = true;
            break;
        }
    }

    // If any source is newer, run the user-supplied transformation.
    if rebuild_needed {
        println!("cargo::warning=Building {:?}", target);
        transform(primary_source, &expanded_sources, target)?;
    }

    Ok(())
}

/// Runs a collection of build tasks.
/// Each task is `(target, sources, transform-fn)`.
/// Target is a &str for convenience, sources is an array of &str,
/// but we convert them to &Path before calling `build_if_out_of_date`.
fn run_tasks(tasks: Vec<(&str, &str, Vec<&str>, fn(&Path, &[PathBuf], &Path) -> io::Result<()>)>) -> io::Result<()> {
    for (target_str, primary_source, sources_str, transform) in tasks {
        let target_path = Path::new(target_str);
        let primary_source_path = Path::new(primary_source);
        // Convert the list of &str into a list of &Path
        let sources_path: Vec<&Path> = sources_str.iter().map(|s| Path::new(*s)).collect();

        build_if_out_of_date(target_path, primary_source_path, &sources_path, transform)?;
    }
    Ok(())
}


// Process the first source file. Supports //!include and //!define directives.
// Include syntax is `//!include path/to/file.wgsl`.
// Define syntax is `//!define NAME VALUE`. This is a simple text replacement.
fn preprocess_shader_file(primary_source: &Path, _sources: &[PathBuf], target: &Path) -> io::Result<()> {
    // We assume the first file is the “main” source.
    let mut simple_defines = HashMap::new();
    let module_string = _preprocess_shader_file(primary_source, &mut simple_defines)?;
    //let mstr = module_string.to_string();
    fs::write(target, module_string)?;
    Ok(())
}

/// Recursively processes a single WGSL file by scanning for special directives.
fn _preprocess_shader_file(path: &Path, simple_defines: &mut HashMap<String, String>) -> io::Result<String> {
    println!("cargo::warning=Processing {:?}", path);
    if !path.is_file() {
        panic!("Shader not found: {:?}", path);
    }

    let mut module_source = String::new();
    BufReader::new(fs::File::open(path)?).read_to_string(&mut module_source)?;

    let mut module_string = String::new();
    let path_string = path.to_str().expect("Could not convert path to string.").to_string();
    module_string.push_str(format!("// Begin {path_str}\n", path_str=path_string).as_str());
    let parent_dir = path.parent().unwrap_or_else(|| Path::new(""));

    for line in module_source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//!include") {
            println!("cargo::warning=Including {:?}", line);
            // Example: `//!include some/other.wgsl`
            // We take each subsequent token as an include path:
            for include_path_str in line.split_whitespace().skip(1) {
                // let include_path = Path::new(include_path_str);
                let include_path = parent_dir.join(include_path_str);
                let included_src = _preprocess_shader_file(&include_path, simple_defines)?;
                module_string.push_str(&included_src);
            }
        } else if trimmed.starts_with("//!define") {
            println!("cargo::warning=Defining {:?}", line);
            let mut tokens = line.split_whitespace();
            tokens.next(); // skip the directive
            let name = tokens
                .next()
                .expect("Expected name after //!define");
            let value = tokens
                .next()
                .expect("Expected value after //!define");
            simple_defines.insert(name.to_string(), value.to_string());
        } else if trimmed.starts_with("//!undef") {
            println!("cargo::warning=Undefining {:?}", line);
            let mut tokens = line.split_whitespace();
            tokens.next(); // skip the directive
            let name = tokens
                .next()
                .expect("Expected name after //!undef");
            simple_defines.remove(name);
        } else {
            println!("cargo::warning=Appending {:?}", line);
            // Expand any defines in this line
            let mut expanded_line = line.to_string();
            for (name, value) in simple_defines.iter() {
                expanded_line = expanded_line.replace(name, value);
            }
            module_string.push_str(&expanded_line);
            module_string.push('\n');
        }
    }

    module_string.push_str(format!("// End {path_str}\n", path_str=path_string).as_str());
    Ok(module_string)
}
