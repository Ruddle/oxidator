use shaderc;

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

#[allow(dead_code)]
pub fn load_old(glsl_code: &str, stage: ShaderStage) -> Vec<u32> {
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
        ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
    };

    wgpu::read_spirv(glsl_to_spirv::compile(&glsl_code, ty).unwrap()).unwrap()
}

use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, ShaderCompilationError>;

// Define our error types. These may be customized for our error handling cases.
// Now we will be able to write our own errors, defer to an underlying error
// implementation, or do something in between.
#[derive(Debug, Clone)]
pub struct ShaderCompilationError {
    pub msg: String,
}

// Generation of an error is completely separate from how it is displayed.
// There's no need to be concerned about cluttering complex logic with the display style.
//
// Note that we don't store any extra info about the errors. This means we can't state
// which string failed to parse without modifying our types to carry that information.
impl fmt::Display for ShaderCompilationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "shader compilation error {}", self.msg)
    }
}

// This is important for other errors to wrap this one.
impl error::Error for ShaderCompilationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

pub fn load(rel_path: &str, stage: ShaderStage) -> Result<Vec<u32>> {
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
