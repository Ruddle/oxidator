use crate::gpu_obj::glsl_compiler;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::slice;

pub fn compile_all_glsl() {
    println!("Compile all glsl");

    let path = std::path::Path::new("./src/shader/");
    let cb = |de: &DirEntry| {
        let path_to_read = de.path();
        let ext = path_to_read.extension().unwrap().to_str().unwrap();

        if !ext.contains("spirv") {
            println!("compiling {:?}", path_to_read);
            let spirv = glsl_compiler::load(path_to_read.to_str().unwrap()).unwrap();

            let file_name = path_to_read.file_name().unwrap();
            let mut path_to_write = path_to_read.parent().unwrap().to_path_buf();
            path_to_write.push("compiled");
            path_to_write.push(file_name);
            let path_to_write = path_to_write.with_extension(format!("{}.spirv", ext));

            println!("write to {:?}", path_to_write);

            let slice_u8: Vec<u8> = spirv
                .iter()
                .map(|w| w.to_le_bytes().iter().copied().collect::<Vec<u8>>())
                .flatten()
                .collect();

            std::fs::write(path_to_write, slice_u8).unwrap();
        }
    };
    visit_dirs(path, &cb).unwrap();
}

fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}
