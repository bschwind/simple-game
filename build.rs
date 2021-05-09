use naga::back::spv;
use std::path::{Path, PathBuf};

const SRC_DIR: &str = "src/graphics/shaders/wgsl";
const COMPILED_DIR: &str = "src/graphics/shaders/compiled";

fn main() {
    println!("cargo:rerun-if-changed={}", SRC_DIR);

    for entry in std::fs::read_dir(SRC_DIR).expect("Shaders directory should exist") {
        let entry = entry.unwrap();
        let path = entry.path();

        if let Some(extension) = path.extension().and_then(|os_str| os_str.to_str()) {
            match extension.to_ascii_lowercase().as_str() {
                "wgsl" => {
                    println!("cargo:rerun-if-changed={}", path.to_string_lossy());
                    compile_shader(path);
                },
                _ => {},
            }
        }
    }
}

fn compile_shader<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let mut output_path = PathBuf::from(COMPILED_DIR);
    output_path.push(path.file_stem().unwrap());
    output_path.set_extension("spv");

    let shader_source = std::fs::read_to_string(path).expect("Shader source should be available");

    let module = naga::front::wgsl::parse_str(&shader_source)
        .map_err(|e| {
            println!("{:#?}", e);
            e
        })
        .unwrap();

    // Output to SPIR-V
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let options = naga::back::spv::Options::default();
    let spv = spv::write_vec(&module, &info, &options).unwrap();

    let bytes = spv.iter().fold(Vec::with_capacity(spv.len() * 4), |mut v, w| {
        v.extend_from_slice(&w.to_le_bytes());
        v
    });

    std::fs::write(output_path, bytes.as_slice()).expect("Couldn't write SPIR-V shader file");
}
