use std::io::{stdout, Write};

pub trait Draw {
    //
}

pub fn draw_output(mode_name: &str, output: &str, viewport_size: (u16, u16)) {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    write!(&mut stdout, "output:\n{}\n----\n", output).unwrap();
}

