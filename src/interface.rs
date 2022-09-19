use anyhow::Result;
use proc_qq::re_exports::ricq::client::event::*;
use proc_qq::re_exports::ricq_core::msg::elem::RQElem;
use proc_qq::re_exports::{bytes, reqwest};
use proc_qq::*;
use rand::Rng;

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::{fs::create_dir, fs::read, fs::read_dir, fs::File};

pub const IMAGE_DIR: &str = "images";

pub const MD5SUMS: &str = ".md5sums";

fn compute_md5sum(buf: &[u8]) -> String {
    format!("{:x}",md5::compute(buf))
}

fn is_image(buf : &[u8]) -> bool {
    fn check(start: &[u8],buf: &[u8]) -> bool {
        start == &buf[0..start.len()]
    }
    // gif jpeg png/apng
    check(&[0x47u8, 0x49u8, 0x46u8],buf) || check(&[0xFF, 0xD8, 0xFF],buf) || check(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],buf)
}

fn count_image(image_dir: impl AsRef<Path>) -> Result<usize> {
    Ok(read_dir(image_dir)?
        .filter(|z| {
            z.as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .ends_with(".image")
        })
        .count())
}

async fn download_image(url: &str) -> Result<bytes::Bytes> {
    Ok(reqwest::ClientBuilder::new()
    .danger_accept_invalid_certs(true)
    .build()?
    .request(reqwest::Method::GET, url)
    .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36")
    .send()
    .await?
    .error_for_status()?
    .bytes()
    .await?)
}

fn compare_md5(source_str: &str, md5_buf: &[u8]) -> Result<bool> {
    let compare = compute_md5sum(md5_buf);
    let mut file = File::open(source_str).or_else(|_| File::create(source_str))?;
    if file.try_clone()?.bytes().count() == 0 {
        writeln!(file,"{}",compare)?;
        return Ok(true);
    }
    let mut reader = BufReader::new(file);
    let mut md5_str = "".into();
    while reader.read_line(&mut md5_str).is_ok() {
        if md5_str == compare {
            return Ok(false);
        }
        md5_str.clear();
    }
    //
    let mut file = OpenOptions::new().append(true).open(source_str)?;
    writeln!(file, "{}", compare)?;
    Ok(true)
}

#[event]
async fn listen(event: &GroupMessageEvent) -> Result<bool> {
    if event.clone().message_content().contains("入典") {
        let message_chain = event.message_chain();

        let image_url = message_chain
            .0
            .iter()
            .find_map(|m| {
                if let RQElem::GroupImage(image) = m.clone().into() {
                    Some(image.url())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        if image_url.is_empty() {
            return Ok(true);
        }

        let buf = match download_image(&image_url).await {
            Ok(b) => b,
            Err(_) => {
                event
                    .send_message_to_source("拉取失败".parse_message_chain())
                    .await?;
                return Ok(true);
            }
        };

        if !is_image(&buf) {
            event
                .send_message_to_source("不支持的格式\n支持的格式为jpeg/jpg,png/apng,gif".parse_message_chain())
                .await?;
            return Ok(true);
        }

        if let Ok(false) = compare_md5(MD5SUMS, &buf) {
            event
                .send_message_to_source(
                    "已检测到与此图片MD5相同的图片,不予添加"
                        .parse_message_chain()
                        .append(event.upload_image_to_source(buf).await?),
                )
                .await?;
            return Ok(true);
        }

        if read_dir(IMAGE_DIR).is_err() {
            create_dir(IMAGE_DIR)?;
        }

        let mut f = File::create(format!(
            "{}/{}.image",
            IMAGE_DIR,
            count_image(IMAGE_DIR)? + 1
        ))?;

        f.write_all(&buf)?;

        event
            .send_message_to_source("添加成功".parse_message_chain())
            .await?;

        Ok(true)
    } else if event.message_content().eq("典") {
        eprintln!("sending message...");
        if read_dir(IMAGE_DIR)?.count() == 0 {
            event
                .send_message_to_source("目前图库里还没有图片".parse_message_chain())
                .await?;
        }
        let img = event
            .upload_image_to_source(read(
                format!(
                    "{}/{}.image",
                    IMAGE_DIR,
                    rand::thread_rng().gen_range(1..=(count_image(IMAGE_DIR)?))
                )
                .as_str(),
            )?)
            .await?;
        event
            .send_message_to_source(img.parse_message_chain())
            .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn module() -> Module {
    module!("listen", "l", listen)
}
