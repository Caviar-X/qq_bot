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
pub const IMAGE_DIR: &str = "static";
pub const MD5SUMS: &str = ".md5sums";
fn compute_md5sum(buf: Vec<u8>) -> Result<String> {
    Ok(String::from_utf8(md5::compute(buf).0.to_vec())?)
}
fn is_jpeg(buf: Vec<u8>) -> bool {
    buf[0] == 0xff_u8
        && buf[1] == 0xd8_u8
        && buf[buf.len() - 2] == 0xff_u8
        && *buf.last().unwrap() == 0xd9_u8
}
fn count_image(image_dir: String) -> Result<usize> {
    Ok(read_dir(image_dir)?
        .filter(|z| {
            z.as_ref()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .ends_with(".jpeg")
        })
        .count())
}
async fn download_image(url: String) -> Result<bytes::Bytes> {
    Ok(reqwest::ClientBuilder::new()
    .danger_accept_invalid_certs(true)
    .build()?
    .request(reqwest::Method::GET, url.as_str())
    .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36")
    .send()
    .await?
    .error_for_status()?
    .bytes()
    .await?)
}
fn compare_md5(source_str: String, md5_buf: Vec<u8>) -> Result<bool> {
    let file = File::open(source_str.clone()).unwrap_or(File::create(source_str.clone())?);
    if file.try_clone()?.bytes().count() == 0 {
        return Ok(true);
    }
    let mut reader = BufReader::new(file);
    let mut md5_str = "".into();
    let compare = compute_md5sum(md5_buf)?;
    while reader.read_line(&mut md5_str).is_ok() {
        if md5_str == compare {
            return Ok(false);
        }
    }
    let mut file = OpenOptions::new().append(true).open(source_str)?;
    writeln!(file, "{}", compare)?;
    Ok(true)
}
#[event]
async fn listen(event: &GroupMessageEvent) -> Result<bool> {
    if event.clone().message_content().contains("入典") {
        let mut image_url = String::new();
        let clone = event.clone();
        let message_chain = clone.message_chain();
        for i in (message_chain.clone()).into_iter() {
            if let RQElem::GroupImage(t) = i {
                image_url = t.url();
            }
        }
        if image_url == String::new() {
            return Ok(true);
        }
        let buf = download_image(image_url).await;
        if buf.is_err() {
            event
                .send_message_to_source("拉取失败".parse_message_chain())
                .await?;
            return Ok(true);
        }
        eprintln!("download finished");
        let buf = buf.unwrap();
        if !is_jpeg(buf.to_vec()) {
            event
                .send_message_to_source("不支持的格式".parse_message_chain())
                .await?;
            return Ok(true);
        }

        if let Ok(false) = compare_md5(MD5SUMS.into(), buf.to_vec()) {
            event
                .send_message_to_source(
                    "已检测到与此图片MD5相同的图片,不予添加"
                        .parse_message_chain()
                        .append(event.upload_image_to_source(buf).await?),
                )
                .await?;
            return Ok(true);
        }
        if !Path::new(IMAGE_DIR).exists() {
            create_dir(IMAGE_DIR)?;
        }
        let mut f = File::create(format!(
            "{}/{}.jpeg",
            IMAGE_DIR,
            count_image(IMAGE_DIR.into())?
        ))?;
        f.write_all(String::from_utf8(buf.to_vec())?.as_bytes())?;
        event
            .send_message_to_source("添加成功".parse_message_chain())
            .await?;
        Ok(true)
    } else if event.message_content() == "来只kf" {
        if read_dir(IMAGE_DIR)?.count() == 0 {
            event
                .send_message_to_source("目前图库里还没有图片".parse_message_chain())
                .await?;
        }
        event
            .upload_image_to_source(read(
                format!(
                    "{}/{}.jpeg",
                    IMAGE_DIR,
                    rand::thread_rng().gen_range(1..=(count_image(IMAGE_DIR.into())?))
                )
                .as_str(),
            )?)
            .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn module() -> Module {
    module!("listen", "l", listen)
}
