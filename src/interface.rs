#![allow(clippy::from_over_into)]

use anyhow::Result;
use proc_qq::re_exports::ricq::client::event::*;
use proc_qq::re_exports::{bytes, reqwest, ricq_core::msg::elem::RQElem};
use proc_qq::*;
use rand::Rng;
use std::io::Write;
use std::path::Path;
use std::{fs::create_dir_all, fs::read, fs::read_dir, fs::remove_file, fs::File};
pub const IMAGE_DIR: &str = "images";
use crate::ghci::*;
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
    Ok(read_dir(image_dir)
        .map(|entries| {
            entries.filter_map(|p| {
                p.ok()?
                    .file_name()
                    .to_str()
                    .and_then(|s| s.split_once('.'))
                    .map(|(name, _)| name.to_string())
            })
        })?
        .collect())
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
                .send_message_to_source(
                    "不支持的格式\n支持的格式为jpeg/jpg,png/apng,gif".parse_message_chain(),
                )
                .await?;
            return Ok(true);
        }
        if read_dir(format!("{}/{}", IMAGE_DIR, event.inner.group_code)).is_err() {
            create_dir_all(format!("{}/{}", IMAGE_DIR, event.inner.group_code))?;
        }
        if File::open(format!(
            "{}/{}/{}.image",
            IMAGE_DIR,
            event.inner.group_code,
            compute_md5sum(&buf)
        ))
        .is_ok()
        {
            event
                .send_message_to_source(
                    "已检测到与此图片MD5相同的图片,不予添加"
                        .parse_message_chain()
                        .append(event.upload_image_to_source(buf.to_vec()).await?),
                )
                .await?;
            return Ok(true);
        }
        let mut f = File::create(format!(
            "{}/{}/{}.image",
            IMAGE_DIR,
            event.inner.group_code,
            compute_md5sum(&buf)
        ))?;
        f.write_all(&buf)?;
        event
            .send_message_to_source("添加成功".parse_message_chain())
            .await?;

        Ok(true)
    } else if event.message_content().eq("典") {
        let res = read_dir(format!("{}/{}", IMAGE_DIR, event.inner.group_code));
        if res.is_err() || res?.count() == 0 {
            event
                .send_message_to_source("目前图库里还没有图片".parse_message_chain())
                .await?;
            return Ok(true);
        }
        let all_file: Vec<String> =
            get_all_md5(format!("{}/{}", IMAGE_DIR, event.inner.group_code))?;
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
    } else if event.clone().message_content().contains("出典") {
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
        if !event
            .client
            .get_group_admin_list(event.inner.group_code)
            .await?
            .get(&event.inner.from_uin)
            .is_some()
        {
            event
                .send_message_to_source("只有管理员才可以出典".parse_message_chain())
                .await?;
            return Ok(true);
        }
        let file = format!(
            "{}/{}/{}.image",
            IMAGE_DIR,
            event.inner.group_code,
            compute_md5sum(&download_image(image_url.as_str()).await?)
        );
        if let Err(e) = remove_file(file) {
            event
                .send_message_to_source("删除失败".parse_message_chain())
                .await?;
            eprintln!("Error : {:#?}", e);
        } else {
            event
                .send_message_to_source("出典成功".parse_message_chain())
                .await?;
        }

        Ok(true)
    } else if event.clone().message_content().starts_with("!ghci") {
        let content = event.message_content();
        let expr = content.get("!ghci".len()..).unwrap_or_default();
        if expr.is_empty() {
            event
                .send_message_to_source("表达式为空或有不合法字符".parse_message_chain())
                .await?;
        }
        let output = execute(expr.into())?;
        if !output.status.success() {
            event
                .send_message_to_source("程序超时".parse_message_chain())
                .await?;
        } else {
            let message = String::from_utf8(output.stdout)?;
            if message.len() >= LIMIT_BYTE {
                event
                    .send_message_to_source(
                        format!("输出大于等于{}B,不予展示", LIMIT_BYTE).parse_message_chain(),
                    )
                    .await?;
            }
            let mut res = String::new();
            for i in message.lines() {
                if i.trim() == "GHCi, version 9.0.2: https://www.haskell.org/ghc/  :? for help" {
                    continue;
                }
                let t: String = i.replace("ghci>", "").trim().into();
                if !t.trim().is_empty() && t.trim() != "Leaving GHCi." {
                    res.push_str(t.as_str());
                    res.push_str("\n");
                }
            }
            event
                .send_message_to_source(res.parse_message_chain())
                .await?;
            event
                .send_message_to_source(String::from_utf8(output.stderr)?.parse_message_chain())
                .await?;
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn module() -> Module {
    module!("listen", "l", listen)
}
