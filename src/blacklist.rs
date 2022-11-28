use crate::interface::OWNER_UIN;
use anyhow::Result;
use proc_qq::re_exports::ricq_core::msg::elem::RQElem;
use proc_qq::*;
use std::collections::HashMap;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
pub const PATH: &'static str = ".blacklist";
pub struct BlackList {
    pub inner: HashMap<i64, Vec<i64>>,
}
impl BlackList {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if File::open(&path).is_err() {
            File::create(path)?;
            return Ok(Self {
                inner: HashMap::new(),
            });
        }
        let mut inner: HashMap<i64, Vec<i64>> = HashMap::new();
        for i in read_to_string(path)?.lines() {
            let (group, uin) = i
                .trim()
                .split_once(' ')
                .map(|(x, y)| (x.parse::<i64>(), y.parse::<i64>()))
                .unwrap();
            let (group, uin) = (group?, uin?);
            match inner.contains_key(&group) {
                true => {
                    inner.get_mut(&group).unwrap().push(uin);
                }
                false => {
                    inner.insert(group, vec![uin]);
                }
            }
        }
        Ok(BlackList { inner })
    }
    pub fn add(&mut self, group: i64, uin: i64) -> Result<()> {
        match self.inner.contains_key(&group) {
            true => {
                self.inner.get_mut(&group).unwrap().push(uin);
            }
            false => {
                self.inner.insert(group, vec![uin]);
            }
        }
        Ok(())
    }
    pub fn contains(&self, group: i64, uin: i64) -> bool {
        if self.inner.contains_key(&group) {
            if self.inner.get(&group).unwrap().contains(&uin) {
                return true;
            }
            return false;
        }
        false
    }
    pub fn remove(&mut self, group: i64, uin: i64) {
        if self.inner.contains_key(&group) {
            let v = self.inner.get_mut(&group).unwrap();
            if let Some(a) = v.iter().position(|&x| x == uin) {
                v.remove(a);
            }
        }
    }
    pub fn rewrite(&self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        let mut file = File::open(path)?;
        for i in self.inner.iter() {
            for j in i.1 {
                writeln!(file, "{} {}", i.0, j)?;
            }
        }
        Ok(())
    }
}
#[event]
async fn blacklist(event: &GroupMessageEvent) -> Result<bool> {
    let chain = &event.message_chain().0;

    let mut blacklist = BlackList::new(PATH)?;
    let mut at = 0;
    for i in chain {
        if let RQElem::At(a) = i.clone().into() {
            at = a.target;
        }
    }
    if at == 0 {
        if let Some(a) = event.message_content().split_whitespace().nth(2) {
            at = a.parse::<i64>().unwrap_or_default();
        }
        event
            .send_message_to_source("未找到at/qq号".parse_message_chain())
            .await?;
    }
    if event.message_content().starts_with("!blacklist") {
        if !event
            .client
            .get_group_admin_list(event.inner.group_code)
            .await?
            .get(&event.inner.from_uin)
            .is_some()
            || event.inner.from_uin != OWNER_UIN
        {
            event
                .send_message_to_source(
                    "只有管理员/Owner才可以对黑名单进行操作".parse_message_chain(),
                )
                .await?;
            return Ok(true);
        }
        match event
            .message_content()
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
        {
            "add" => {
                blacklist.add(event.inner.group_code, event.from_uin())?;
                blacklist.rewrite(PATH)?;
                return Ok(true);
            }
            "remove" => {
                blacklist.remove(event.inner.group_code, event.from_uin());
                blacklist.rewrite(PATH)?;
            }
            _ => {
                event
                    .send_message_to_source("未知的参数".parse_message_chain())
                    .await?;
            }
        }
        return Ok(true);
    }
    Ok(false)
}
pub fn module() -> Module {
    module!("blaklist", "b", blacklist)
}
