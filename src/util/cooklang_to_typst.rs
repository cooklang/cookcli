use anyhow::Result;
use cooklang::{
    convert::Converter,
    //model::{Item, Section, Step},
    Recipe,
};
use std::io;

pub fn print_typst(
    _recipe: &Recipe,
    _name: &str,
    _scale: f64,
    _converter: &Converter,
    mut writer: impl io::Write,
) -> Result<()> {
    writeln!(writer, r"Hello, World!")?;

    Ok(())
}