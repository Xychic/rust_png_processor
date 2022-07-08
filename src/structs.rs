use std::{
    fmt::Debug,
    fs::File,
    io::{Read, Write},
    slice::Iter,
};

use libflate::zlib::Encoder;

#[derive(Clone)]
pub struct Data {
    pub data: Vec<u8>,
}

impl Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data_chunks = format!(
            "[\n{}\n]",
            self.data
                .chunks(16)
                .map(|data_chunk| {
                    format!(
                        "\t{}",
                        data_chunk
                            .iter()
                            .map(|x| format!("0x{:02X?}", x))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n")
        );
        write!(f, "{}", data_chunks)
    }
}

#[derive(Clone)]
pub struct Chunk {
    pub size: u32,
    pub name: String,
    pub data: Data,
    pub crc: u32,
}

impl Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Chunk {{
    Size:\t {}
    Name:\t\"{}\"
    Data:\t{}
    CRC:\t{:02X?}
}}",
            self.size,
            self.name,
            format!("{:?}", self.data).replace('\n', "\n\t\t"),
            self.crc
        )
    }
}

impl Chunk {
    pub fn crc(name: &str, data: &[u8]) -> u32 {
        let mut all_data = Vec::with_capacity((4 + data.len()) as usize);
        for byte in ascii_to_bytes(&name) {
            all_data.push(byte);
        }
        for &byte in data {
            all_data.push(byte);
        }
        crc32fast::hash(&all_data)
    }

    pub fn check_crc(&self) -> bool {
        return Chunk::crc(&self.name, &self.data()) == self.crc;
    }

    pub fn from_data(name: &str, data: &[u8]) -> Chunk {
        Chunk {
            size: data.len() as u32,
            name: name.to_owned(),
            data: Data {
                data: data.to_owned(),
            },
            crc: Chunk::crc(name, data),
        }
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data.data
    }

    pub fn chunk_size(&self) -> usize {
        self.size as usize + 8
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(self.size as usize + 12);
        for byte in self.size.to_be_bytes() {
            res.push(byte);
        }
        for char in self.name.chars() {
            res.push(char as u8);
        }
        for &byte in self.data() {
            res.push(byte);
        }
        for byte in self.crc.to_be_bytes() {
            res.push(byte);
        }
        res
    }
}

fn ascii_to_bytes(ascii: &str) -> Vec<u8> {
    ascii.chars().map(|c| c as u8).collect()
}
pub enum ColourType {
    Grayscale,
    RGB,
    Palette,
    GrayscaleAlpha,
    RGBAlpha,
}

impl ColourType {
    pub fn valid_bit_depth(&self, bit_depth: u8) -> bool {
        match self {
            ColourType::Grayscale => [1, 2, 4, 8, 16].contains(&bit_depth),
            ColourType::RGB => [8, 16].contains(&bit_depth),
            ColourType::Palette => [1, 2, 4, 8].contains(&bit_depth),
            ColourType::GrayscaleAlpha => [8, 16].contains(&bit_depth),
            ColourType::RGBAlpha => [8, 16].contains(&bit_depth),
        }
    }

    pub fn get_code(&self) -> u8 {
        match self {
            ColourType::Grayscale => 0,
            ColourType::RGB => 2,
            ColourType::Palette => 3,
            ColourType::GrayscaleAlpha => 4,
            ColourType::RGBAlpha => 6,
        }
    }
}

const HEADER: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Debug)]
pub struct PNG {
    header: [u8; 8],
    pub ihdr: Chunk,
    pub dimension: (u32, u32),
    pub bit_depth: u8,
    pub chunks: Vec<Chunk>,
    pub data: Vec<Vec<u8>>,
    end: Chunk,
}

impl PNG {
    // pub fn to_bytes(&self) -> Vec<u8> {
    //     let mut total_size =
    //     let mut result = Vec::with_capacity(capacity)
    //     todo!()
    // }

    pub fn new(width: u32, height: u32, bit_depth: u8, colour_type: ColourType) -> PNG {
        assert!(colour_type.valid_bit_depth(bit_depth));

        let width_bytes = width.to_be_bytes();
        let height_bytes = height.to_be_bytes();

        let bits_per_row = width as usize * bit_depth as usize;
        let bytes_per_row = bits_per_row / 8 + if bits_per_row % 8 != 0 { 1 } else { 0 };

        PNG {
            header: HEADER,
            ihdr: Chunk::from_data(
                "IHDR",
                &vec![
                    width_bytes[0],
                    width_bytes[1],
                    width_bytes[2],
                    width_bytes[3],
                    height_bytes[0],
                    height_bytes[1],
                    height_bytes[2],
                    height_bytes[3],
                    bit_depth,
                    colour_type.get_code(),
                    0, // Type 0 compression
                    0, // Type 0 filtering
                    0, // No interlacing
                ],
            ),
            bit_depth,
            end: Chunk {
                size: 0,
                name: "IEND".to_string(),
                data: Data { data: Vec::new() },
                crc: 0xAE426082,
            },
            chunks: Vec::new(),
            data: vec![vec![0xFF; bytes_per_row]; height as usize],
            dimension: (width, height),
        }
    }

    pub fn from_file(filepath: &str) -> Result<PNG, &str> {
        let mut file = File::open(filepath).expect("error opening file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("error reading file");

        let mut size = buffer.len();

        let mut buffer_iter = buffer.iter();

        let header_buf = take_bytes(&mut buffer_iter, 8);
        size -= 8; // For the header

        println!("Header: {:02X?}", header_buf);
        if header_buf == HEADER {
            println!("Header check passed.");
        } else {
            println!("Header check failed!");
        }

        let mut chunks = Vec::new();

        while size > 0 {
            let (chunk, chunk_size) = get_chunk(&mut buffer_iter);
            size -= chunk_size;
            chunks.push(chunk);
        }

        let ihdr = &chunks[0];
        if ihdr.name != "IHDR" {
            return Err("First chunk must be IHDR!");
        } else if ihdr.size != 13 {
            return Err("IHDR chunk wrongs size!");
        } else if !ihdr.check_crc() {
            return Err("IHDR CRC failed!");
        }

        let ihdr_data = ihdr.data();
        let mut ihdr_iter = ihdr_data.iter();
        let width = bytes_to_u32(&take_bytes(&mut ihdr_iter, 4));
        let height = bytes_to_u32(&take_bytes(&mut ihdr_iter, 4));

        let bit_depth = *ihdr_iter.next().unwrap();
        let colour_type = *ihdr_iter.next().unwrap();

        let width_bytes = width.to_be_bytes();
        let height_bytes = height.to_be_bytes();

        let chunk_count = chunks.len();

        Ok(PNG {
            header: HEADER,
            ihdr: Chunk::from_data(
                "IHDR",
                &vec![
                    width_bytes[0],
                    width_bytes[1],
                    width_bytes[2],
                    width_bytes[3],
                    height_bytes[0],
                    height_bytes[1],
                    height_bytes[2],
                    height_bytes[3],
                    bit_depth,
                    colour_type,
                    0, // Type 0 compression
                    0, // Type 0 filtering
                    0, // No interlacing
                ],
            ),
            bit_depth,
            end: Chunk {
                size: 0,
                name: "IEND".to_string(),
                data: Data { data: Vec::new() },
                crc: 0xAE426082,
            },
            chunks: chunks[1..(chunk_count - 1)].to_vec(),
            data: Vec::new(),
            dimension: (width, height),
        })
    }

    pub fn save(&self, filepath: &str) {
    println!("Saving...");

        let mut file = File::create(filepath).unwrap();
        file.write_all(&self.header).unwrap();
        file.write_all(&self.ihdr.to_bytes()).unwrap();

        for chunk in &self.chunks {
            file.write_all(&chunk.to_bytes()).unwrap();
        }

        let row_len = self.data[0].len();
        let total_ops = self.data.len() * (row_len + 1);
        let mut uncompressed = Vec::with_capacity(total_ops);

        for row in &self.data {
            uncompressed.push(0x00); // No filter
            for &byte in row {
                uncompressed.push(byte);
            }
        }
        println!("Compressing data...");
        let data = Self::compress_block(&uncompressed);
        println!("Writing chunks...");
        for block in data.chunks(1024) {
            file.write_all(&Chunk::from_data("IDAT", block).to_bytes())
                .unwrap();
        }
        file.write_all(&self.end.to_bytes()).unwrap();

    println!("Saved!");

    }

    fn compress_block(data: &[u8]) -> Vec<u8> {
        let mut encoder = Encoder::new(Vec::new()).unwrap();
        encoder.write_all(data).unwrap();
        encoder.finish().into_result().unwrap()
    }

    pub fn put_pixel(&mut self, pixel_data: u8, x: u32, y: u32) {
        if pixel_data & 0x01 == 1 {
            self.data[y as usize][x as usize / 8] |= 0x80 >> (x % 8);
        } else {
            self.data[y as usize][x as usize / 8] &= !(0x80 >> (x % 8));
        }
    }
}

fn get_chunk(buffer: &mut Iter<u8>) -> (Chunk, usize) {
    let chunk_length_buf = take_bytes(buffer, 4);
    let chunk_length = bytes_to_u32(&chunk_length_buf);

    let chunk_name_buf = take_bytes(buffer, 4);
    let chunk_name = bytes_to_ascii(&chunk_name_buf);

    let data = take_bytes(buffer, chunk_length);
    let crc_buf = take_bytes(buffer, 4);
    let crc = bytes_to_u32(&crc_buf);

    (
        Chunk {
            size: chunk_length,
            name: chunk_name,
            data: Data { data },
            crc,
        },
        chunk_length as usize + 12,
    )
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0, |acc, &x| (acc << 8) + x as u32)
}

fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes.iter().map(|&x| x as char).collect()
}

fn take_bytes(buffer: &mut Iter<u8>, count: u32) -> Vec<u8> {
    (0..count).map(|_| *buffer.next().unwrap()).collect()
}
