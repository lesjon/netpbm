use std::{env, fs, io};
use std::error::Error;
use std::io::ErrorKind;
use netpbm::display;
use netpbm::parser;

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<String>>();
    let image_file = args.get(1)
        .ok_or_else(|| Box::new(io::Error::new(ErrorKind::InvalidInput, "Not enough arguments!")))?;
    let contents = fs::read(image_file)?;
    let image = parser::parse(&contents)?;
    display::display_netpbm(&image.data, image.width, image.height, image.max_value);
    Ok(())
}
