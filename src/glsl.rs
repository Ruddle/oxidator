mod glsl_compiler;

use crate::glsl_compiler::*;

use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::slice;

fn main() {
    println!("Compile all glsl");

    let path = std::path::Path::new("./src/shader/");
    let cb = |de: &DirEntry| {
        let path = de.path();
        let ext = path.extension().unwrap().to_str().unwrap();

        if !ext.contains("spirv") {
            println!("{:?}", de);
            let spirv = glsl_compiler::load(de.path().to_str().unwrap()).unwrap();

            let path2 = path.with_extension(format!("{}.spirv", ext));

            let slice_u8: Vec<u8> = spirv
                .iter()
                .map(|w| w.to_le_bytes().iter().copied().collect::<Vec<u8>>())
                .flatten()
                .collect();

            std::fs::write(path2, slice_u8).unwrap();
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
