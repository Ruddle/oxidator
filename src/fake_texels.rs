pub fn create_texels(size: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let i = i as f32 / size as f32;
            let j = j as f32 / size as f32;
            v.push((i * 255.0) as u8);
            v.push(((1.0 - j) * 255.0) as u8);
            v.push(((1.0 - i) * j * 255.0) as u8);
            v.push(255);
        }
    }
    v
}
