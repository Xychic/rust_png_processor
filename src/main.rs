#[rustfmt::skip::macros(vec)]
mod structs;

use structs::{ColourType, PNG};

fn main() {
    let width = 250_000;
    let height = 250_000;
    let mut image = PNG::new(width, height, 1, ColourType::Grayscale);

    for index in 0..width.min(height) {
        image.put_pixel(0, index, index);
        image.put_pixel(0, width - 1 - index, index);
    }

    image.save("./test_copied.png");
}
