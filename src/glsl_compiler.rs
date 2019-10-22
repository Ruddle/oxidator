#[cfg(feature = "use_shaderc")]
use shaderc;

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, ShaderCompilationError>;

#[derive(Debug, Clone)]
pub struct ShaderCompilationError {
    pub msg: String,
}

impl fmt::Display for ShaderCompilationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "shader compilation error {}", self.msg)
    }
}

impl error::Error for ShaderCompilationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(not(feature = "use_shaderc"))]
pub fn load(rel_path: &str, stage: ShaderStage) -> Result<Vec<u32>> {
    log::debug!("glsl_to_spirv : compiling {}", rel_path);
    let glsl_code = load_str!(rel_path);
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
        ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
    };

    Ok(
        wgpu::read_spirv(glsl_to_spirv::compile(&glsl_code, ty).map_err(|e| {
            ShaderCompilationError {
                msg: format!("{}", e),
            }
        })?)
        .map_err(|e| ShaderCompilationError {
            msg: format!("{}", e),
        })?,
    )
    //   wgpu::read_spirv(glsl_to_spirv::compile(&glsl_code, ty).unwrap()).unwrap()
}

#[cfg(feature = "use_shaderc")]
pub fn load(rel_path: &str, stage: ShaderStage) -> Result<Vec<u32>> {
    log::debug!("shaderc : compiling {}", rel_path);
    let glsl_code = load_str!(rel_path);

    let ty = match stage {
        ShaderStage::Vertex => shaderc::ShaderKind::Vertex,
        ShaderStage::Fragment => shaderc::ShaderKind::Fragment,
        ShaderStage::Compute => shaderc::ShaderKind::Compute,
    };

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.add_macro_definition("EP", Some("main"));
    let binary_result = compiler
        .compile_into_spirv(glsl_code, ty, rel_path, "main", Some(&options))
        .map_err(|e| ShaderCompilationError {
            msg: format!("{}", e),
        })?;

    Ok(binary_result.as_binary().to_owned())
}
