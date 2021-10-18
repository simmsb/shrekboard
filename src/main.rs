use std::{fs::File, path::PathBuf, thread::sleep};

use color_eyre::{eyre::Result, Help};
use hidapi::HidDevice;
use image::{
    imageops::{dither, grayscale, resize, BiLevel},
    AnimationDecoder, GenericImage, GenericImageView, ImageBuffer, Luma, Pixel,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "shrekboard")]
struct Opt {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    // #[structopt(parse(from_os_str))]
    // kb: PathBuf,
}

fn get_kb() -> Result<HidDevice> {
    let hid_api = hidapi::HidApi::new()?;
    for dev_info in hid_api.device_list() {
        // println!("{:?}", (dev_info.vendor_id(), dev_info.product_id(),
        //     dev_info.usage_page(), dev_info.usage(), dev_info.path(), dev_info.interface_number()));
        if (
            dev_info.vendor_id(),
            dev_info.product_id(),
            dev_info.interface_number(),
        ) == (0xFC32, 0x0287, 1)
        {
            return Ok(dev_info.open_device(&hid_api)?);
        }
    }

    Err(color_eyre::eyre::eyre!("Couldn't find the keyboard :("))
}

// struct __attribute__((__packed__)) display_packet {
//     bool    to_master;
//     uint8_t offset;
//     uint8_t length;
//     uint8_t buf[];
// };

fn emit_image(im: &ImageBuffer<Luma<u8>, Vec<u8>>, dev: &HidDevice) -> Result<()> {
    let mut lhs = vec![0xffu8; 512];
    let mut rhs = vec![0xffu8; 512];

    // let mut lhs = vec![
    //     0u8,  0, 18,128, 32, 10, 64, 18,132, 48,  5, 64, 20,193,  4, 16, 65,  0,132,  0,128,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,192,  0, 77, 31, 79,183,  5, 79,151, 43,199, 21, 47, 87,239,252,191,244,188,236,120,244,248,192,240,160,128,128,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    //     0,  0,  0,170,  0,  0, 21,162,  0,  4, 65,  8,130, 32,140, 65,144, 42,128, 52,128, 36,  0, 73,  0,  0,  0,128,128,  0,192,192,128,  0,  0,  0,  0,  0,  0,  0,  0,128,128,192,224,176,224,242,224,100,160, 68,208,180, 73, 54,192,125,  0,107,132,240,141, 34,  8,230,  8, 85,213,255,255,221,255,255,187,255,255,191,253,119,255,255,126,247,222,124,248,232,192,160,  0,  0, 64,128, 32,  0, 64,144, 32, 64, 80,144,108,  0, 72,144,228,184,104,216,176, 96,128, 96,192,160, 64,128,  0,128,  0,  0,  0,  0,  0,  0,  0,  0,
    //     0,128,  0, 10,160,  0,196,240,244,255,255,247,255,191,255,255,247,254,254,254,246,190,255,254,255,247,190,255,255,255,171,255,110,221,246, 61,238,251,214,125,223,243, 94,255,247,190,253,215,127,252,234,179,106,244,253,203, 50,196,253, 63,202,162,212,253,189,226,205,240,215,127,255,238,187,255,109,255, 59,183, 63, 31,173, 15, 43, 67, 11, 83,  1, 87,128, 21,161, 11, 64, 21,192, 21, 41, 66,144,  5, 32, 77,  0, 81,  4, 41,130, 45,  3,254, 37,219,172,115,140,119,200, 63,193,190, 81,175, 84,184, 96,128,  0,  0,
    //     0,  0, 32,  4, 84,255,255,255,255,254,255,247,255,255,255,191,142,151,167,135,135, 23,  7, 95, 15, 31, 31, 63,191, 62,251,159,213,223,181, 31, 55, 28, 55, 61, 47,123,255,213,255,191,246,223,253,183,255,255,213,255,255,219,255,191,251,215,255,190,251,239,254,187,255,214,255,189,127,  3, 75,  1, 82,  8, 66, 16, 68,  1, 84,  0, 85,  0, 85,  0, 85,160, 10, 64, 20, 75, 32, 74,  0, 84,  9, 82,  0, 84,137,210,128, 52,129, 40,  2, 80,  4, 81,239,148,107,221, 34,223,104,151,104,223, 32,255,  1,254, 69,190, 72,  0,
    // ];

    for (x, y, p) in im.enumerate_pixels() {
        let on_rhs = y > 31;
        let y = if on_rhs { y - 32 } else { y };
        let idx = x + (y / 8) * 128;

        //println!("x: {}, y: {}, idx: {}", x, y, idx);

        let val = p.0[0] > 127;

        let buf = if on_rhs { &mut rhs } else { &mut lhs };
        if val {
            buf[idx as usize] |= 1 << (y % 8);
        } else {
            buf[idx as usize] &= !(1 << (y % 8));
        }
    }

    // let lines = 64 / 8;

    // for line in 0..lines {
    //     for y in 0..128 {
    //         let mut v = 0;
    //         for i in 0..4 {
    //             let x = line * 8 + i;
    //             let b = if im.get_pixel(x, y).0[0] > 127 { 1 } else { 0 };
    //             v |= b << i;
    //         }
    //         for i in 0..4 {
    //             let x = line * 8 + i + 1;
    //             let b = if im.get_pixel(x, y).0[0] > 127 { 1 } else { 0 };
    //             v |= b << (i + 4);
    //         }
    //         if line < 4 {
    //             lhs.push(v);
    //         } else {
    //             rhs.push(v);
    //         }
    //     }
    // }

    const CHUNK_SIZE: usize = 24;
    const HEADER_LEN: usize = 7;

    let mut buf = vec![0u8; CHUNK_SIZE + HEADER_LEN];

    println!("bufs: {}, {}", lhs.len(), rhs.len());

    for (i, chunk) in lhs.chunks(CHUNK_SIZE).enumerate() {
        let buf = &mut buf[0..(HEADER_LEN + chunk.len())];
        buf[0] = 0;
        buf[1] = 1;
        buf[2..4].copy_from_slice(&(i as u16 * 24).to_le_bytes());
        buf[4..6].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
        buf[6] = 1;
        buf[7..].copy_from_slice(chunk);

        // write twice for some reason?
        dev.write(&buf)?;
        dev.write(&buf)?;
    }

    for (i, chunk) in rhs.chunks(CHUNK_SIZE).enumerate() {
        let buf = &mut buf[0..(HEADER_LEN + chunk.len())];
        buf[0] = 0;
        buf[1] = 1;
        buf[2..4].copy_from_slice(&(i as u16 * 24).to_le_bytes());
        buf[4..6].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
        buf[6] = 0;
        buf[7..].copy_from_slice(chunk);

        // write twice for some reason?
        dev.write(&buf)?;
        dev.write(&buf)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let opt = Opt::from_args();
    let kb = get_kb()?;

    let gif = File::open(&opt.file).section("Couldn't find your gif")?;

    let decoder = image::gif::GifDecoder::new(gif).section("Are you sure this is a gif?")?;

    for frame in decoder.into_frames() {
        let frame = frame.section("Some frame is borked")?;
        sleep(frame.delay().into());

        let mut image = grayscale(&resize(
            &image::imageops::rotate90(frame.buffer()),
            128,
            64,
            image::imageops::FilterType::Lanczos3,
        ));
        dither(&mut image, &BiLevel);
        emit_image(&image, &kb)?;
        println!("yo ok");
    }

    Ok(())
}
