use anyhow::Result;
use proc_qq::re_exports::ricq::client::event::*;
use proc_qq::re_exports::{bytes, reqwest,ricq_core::msg::elem::RQElem};
use proc_qq::*;
use rand::Rng;
use std::io::Write;
use std::path::Path;
use std::{fs::create_dir, fs::create_dir_all, fs::read, fs::read_dir, fs::File};

pub const IMAGE_DIR: &str = "images";

fn compute_md5sum(buf: &[u8]) -> String {
    format!("{:x}", md5::compute(buf))
}

fn is_image(buf: &[u8]) -> bool {
    fn check(start: &[u8], buf: &[u8]) -> bool {
        start == &buf[0..start.len()]
    }
    // gif jpeg png/apng
    check(&[0x47u8, 0x49u8, 0x46u8], buf)
        || check(&[0xFF, 0xD8, 0xFF], buf)
        || check(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], buf)
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

fn get_all_md5(image_dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let mut all: Vec<String> = Vec::new();
    for i in read_dir(image_dir)? {
        let i: String = i?.file_name().into_string().unwrap_or_default();
        if !i.is_empty() {
            all.push(
                i.get(..i.chars().position(|x| x == '.').unwrap_or_default())
                    .unwrap_or_default()
                    .into(),
            );
        }
    }
    Ok(all
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<String>>())
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
            }).unwrap_or_default();
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
                .send_message_to_source(
                    "不支持的格式\n支持的格式为jpeg/jpg,png/apng,gif".parse_message_chain(),
                )
                .await?;
            return Ok(true);
        }
        if read_dir(format!("{}/{}",IMAGE_DIR,event.inner.group_code)).is_err() {
            create_dir_all(format!("{}/{}",IMAGE_DIR,event.inner.group_code))?;
        }
        if File::open(format!("{}/{}/{}.image", IMAGE_DIR,event.inner.group_code, compute_md5sum(&buf))).is_ok() {
            event
                .send_message_to_source(
                    "已检测到与此图片MD5相同的图片,不予添加"
                        .parse_message_chain()
                        .append(event.upload_image_to_source(buf.to_vec()).await?),
                )
                .await?;
            return Ok(true);
        }
        let mut f = File::create(format!("{}/{}/{}.image", IMAGE_DIR,event.inner.group_code, compute_md5sum(&buf)))?;
        f.write_all(&buf)?;
        event
            .send_message_to_source("添加成功".parse_message_chain())
            .await?;

        Ok(true)
    } else if event.message_content().eq("典") {
        if read_dir(format!("{}/{}",IMAGE_DIR,event.inner.group_code))?.count() == 0 {
            event
                .send_message_to_source("目前图库里还没有图片".parse_message_chain())
                .await?;
        }
        let all_file: Vec<String> = get_all_md5(format!("{}/{}",IMAGE_DIR,event.inner.group_code))?;
        let img = event
            .upload_image_to_source(read(format!(
                "{}/{}/{}.image",
                IMAGE_DIR,
                event.inner.group_code,
                all_file[rand::thread_rng().gen_range(0..all_file.len())]
            ))?)
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
