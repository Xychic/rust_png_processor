mod structs;

use structs::{Chunk, ColourType, PNG};

fn main() {
    let image = PNG::from_file("./sample.png").unwrap();
    image.save("./sample-saved.png");

    let test_blank_rgb_image = PNG::new(200, 300, 8, ColourType::RGB);
    test_blank_rgb_image.save("./test_blank_rgb_image.png");

    let mut test_greyscale_image = PNG::new(100, 100, 8, ColourType::Grayscale);
    test_greyscale_image
        .data
        .push(Chunk::from_data("bKGD", &vec![0x00, 0xFF]));
    test_greyscale_image.save("test_greyscale_image.png")
}
