mod structs;

use structs::{ColourType, PNG};

fn main() {
    let image = PNG::from_file("./sample.png").unwrap();
    image.save("./sample-saved.png");

    let test_blank_rgb_image = PNG::new(200, 300, 8, ColourType::RGB);
    test_blank_rgb_image.save("./test_blank_rgb_image.png");
}
