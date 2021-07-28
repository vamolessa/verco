use std::io::{stdout, Write};

pub trait Draw {
    //
}

pub fn draw_output(mode_name: &str, output: &str) {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    write!(&mut stdout, "output:\n{}", output).unwrap();
}

