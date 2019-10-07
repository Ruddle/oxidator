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

pub fn checker(size: usize) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let pair = (i + j) % 2 == 0;
            if pair {
                v.push(0);
                v.push(0);
                v.push(0);
            } else {
                v.push(255);
                v.push(255);
                v.push(255);
            }
            v.push(255);
        }
    }
    v
}
