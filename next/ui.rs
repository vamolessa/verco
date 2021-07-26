use std::{
    fmt,
    io::{stdout, Write},
};

use crossterm;

use crate::application::ActionKind;

pub fn draw_output(action: &str, output: &str) {
    let stdout = stdout();
    let mut stdout = stdout.lock();

    write!(&mut stdout, "output:\n{}", output).unwrap();
}

