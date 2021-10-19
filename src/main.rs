use std::{fs::File, io::{Seek, SeekFrom}, path::PathBuf, thread::sleep};

use color_eyre::{eyre::Result, Help};
use hidapi::HidDevice;
use image::{
    imageops::{dither, grayscale, resize, BiLevel},
    AnimationDecoder, ImageBuffer, Luma,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "shrekboard")]
struct Opt {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    // #[structopt(parse(from_os_str))]
    // kb: PathBuf,
    #[structopt(long, short)]
    r#loop: bool,
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
    let mut lhs = vec![0x00u8; 512];
    let mut rhs = vec![0x00u8; 512];

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

    let mut buf = vec![0u8; 33];

    println!("bufs: {}, {}", lhs.len(), rhs.len());

    for (i, chunk) in lhs.chunks(CHUNK_SIZE).enumerate() {
        let tbuf = &mut buf[0..(HEADER_LEN + chunk.len())];
        tbuf[0] = 0;
        tbuf[1] = 1;
        tbuf[2..4].copy_from_slice(&(i as u16 * 24).to_le_bytes());
        tbuf[4..6].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
        tbuf[6] = 1;
        tbuf[7..].copy_from_slice(chunk);

        dev.write(&buf)?;
        //dev.write(&buf)?;
    }

    for (i, chunk) in rhs.chunks(CHUNK_SIZE).enumerate() {
        let tbuf = &mut buf[0..(HEADER_LEN + chunk.len())];
        tbuf[0] = 0;
        tbuf[1] = 1;
        tbuf[2..4].copy_from_slice(&(i as u16 * 24).to_le_bytes());
        tbuf[4..6].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
        tbuf[6] = 0;
        tbuf[7..].copy_from_slice(chunk);

        dev.write(&buf)?;
        //dev.write(&buf)?;
    }

    buf[0] = 0;
    buf[1] = 2;
    dev.write(&buf)?;
    dev.write(&buf)?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let opt = Opt::from_args();
    let kb = get_kb()?;

    let mut gif = File::open(&opt.file).section("Couldn't find your gif")?;

    loop {
        let decoder = image::gif::GifDecoder::new(&gif).section("Are you sure this is a gif?")?;

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

        if !opt.r#loop {
            break;
        }

        gif.seek(SeekFrom::Start(0))?;
    }

    Ok(())
}
