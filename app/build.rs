use std::{
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{self, prelude::*},
    path::Path,
};

use shaderc::{CompileOptions, Compiler, OptimizationLevel, ShaderKind};

const SHADER_PATH: &str = "../shaders/";

fn main() -> Result<(), Box<dyn Error>> {
    let mut compiler = Compiler::new().ok_or("Error trying to initialize shader compiler")?;

    let compile_options = {
        let mut compile_options =
            CompileOptions::new().ok_or("Error trying to initialize shader compile options")?;

        #[cfg(debug_assertions)]
        let optimization_level = OptimizationLevel::Zero;
        #[cfg(not(debug_assertions))]
        let optimization_level = OptimizationLevel::Performance;

        compile_options.set_optimization_level(optimization_level);

        #[cfg(debug_assertions)]
        compile_options.set_generate_debug_info();

        #[cfg(debug_assertions)]
        compile_options.set_warnings_as_errors();

        compile_options
    };

    for entry in fs::read_dir(SHADER_PATH)? {
        let entry = entry?;
        let file_path = match entry.file_type()? {
            file_type if file_type.is_file() => entry.path().canonicalize()?,
            file_type if file_type.is_symlink() => match entry.path().canonicalize()? {
                path if path.is_file() => path,
                _ => continue,
            },
            _ => continue,
        };

        match file_path.extension() {
            Some(str) if str.is_empty() => continue,
            None => continue,
            _ => (),
        }

        let shader_kind = match get_shader_kind(&file_path) {
            Some(shader_kind) => shader_kind,
            None => continue,
        };

        println!("cargo:rerun-if-changed={}", file_path.display());

        let source_text = read_file(&file_path)?;
        let file_name = file_path.file_name().unwrap(); // Safe since we're sure it's a file

        let compilation_artifact = compiler.compile_into_spirv(
            &source_text,
            shader_kind,
            &file_name.to_string_lossy(),
            "main",
            Some(&compile_options),
        )?;

        #[cfg(debug_assertions)]
        if compilation_artifact.get_num_warnings() > 0 {
            return Err(compilation_artifact.get_warning_messages().into());
        }

        let output_path = {
            let extension = file_path.extension().unwrap(); // Safe because we're sure it's a file and has an extension
            file_path.with_extension(Path::new(extension).with_extension("spv"))
        };

        println!("cargo:rerun-if-changed={}", output_path.display());

        write_binary_file(&output_path, compilation_artifact.as_binary_u8())?;
    }

    Ok(())
}

fn get_shader_kind<P: AsRef<Path>>(path: P) -> Option<ShaderKind> {
    let extension = path.as_ref().extension()?;

    match extension {
        // Normal shaders
        stage if stage == OsStr::new("vert") => Some(ShaderKind::DefaultVertex),
        stage if stage == OsStr::new("frag") => Some(ShaderKind::DefaultFragment),
        stage if stage == OsStr::new("comp") => Some(ShaderKind::DefaultCompute),
        stage if stage == OsStr::new("geom") => Some(ShaderKind::DefaultGeometry),
        stage if stage == OsStr::new("tesc") => Some(ShaderKind::DefaultTessControl),
        stage if stage == OsStr::new("tese") => Some(ShaderKind::DefaultTessEvaluation),
        // Raytracing shaders
        stage if stage == OsStr::new("rgen") => Some(ShaderKind::DefaultRayGeneration),
        stage if stage == OsStr::new("rahit") => Some(ShaderKind::DefaultAnyHit),
        stage if stage == OsStr::new("rchit") => Some(ShaderKind::DefaultClosestHit),
        stage if stage == OsStr::new("rmiss") => Some(ShaderKind::DefaultMiss),
        stage if stage == OsStr::new("rint") => Some(ShaderKind::DefaultIntersection),
        stage if stage == OsStr::new("rcall") => Some(ShaderKind::DefaultCallable),
        // Mesh shaders
        stage if stage == OsStr::new("task") => Some(ShaderKind::DefaultTask),
        stage if stage == OsStr::new("mesh") => Some(ShaderKind::DefaultMesh),
        _ => None,
    }
}

fn read_file<P: AsRef<Path>>(path: P) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(path)?;

    let mut code = Vec::new();
    file.read_to_end(&mut code)?;

    let data = String::from_utf8(code)?;

    Ok(data)
}

fn write_binary_file<P: AsRef<Path>>(path: P, data: &[u8]) -> io::Result<()> {
    let mut file: File = File::create(path)?;

    file.write_all(data)?;

    Ok(())
}
