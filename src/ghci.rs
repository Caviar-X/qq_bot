use anyhow::*;
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions};
use bollard::exec::*;
use bollard::*;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::string::String;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn;
const CONTAINER_NAME: &'static str = "ghci_container";
const IMAGE_NAME: &'static str = "archlinux";
const LIMIT_BYTE: usize = 1000;
macro_rules! hashmap {
    ($($k : expr, $v : expr),*) => {
        {
            let mut t = HashMap::new();
            $(t.insert($k,$v);)*
            t
        }
    };
}
pub async fn is_container_running(id: String) -> Result<bool> {
    Ok(!Docker::connect_with_local_defaults()?
        .list_containers(Some(ListContainersOptions::<String> {
            filters: hashmap!("id".into(), vec![id]),
            ..Default::default()
        }))
        .await?
        .is_empty())
}
pub async fn get_id_by_name(name: String) -> Result<String> {
    Ok(Docker::connect_with_local_defaults()?
        .list_containers(Some(ListContainersOptions::<String> {
            filters: hashmap!("name".into(), vec![name]),
            ..Default::default()
        }))
        .await?
        .first()
        .ok_or_else(|| anyhow!("Cannot find match id!"))?
        .id
        .as_ref()
        .ok_or_else(|| anyhow!("Error occoured while getting id"))?
        .into())
}
pub async fn execute(expr: String) -> Result<String> {
    let docker = Docker::connect_with_local_defaults()?;
    let id;
    if !is_container_running(get_id_by_name(CONTAINER_NAME.into()).await?).await? {
        return Err(anyhow!("Match container is not running!"));
    } else {
        id = get_id_by_name(CONTAINER_NAME.into()).await?;
    }
    let exec = docker
        .create_exec(
            id.as_str(),
            CreateExecOptions {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(vec!["ghci"]),
                ..Default::default()
            },
        )
        .await?
        .id;
    let mut output = String::new();
    if let StartExecResults::Attached {
        input: mut i,
        output: mut o,
    } = docker.start_exec(&exec, None).await?
    {
        spawn(async move {
            i.write_all(expr.as_bytes()).await.ok();
        });
        while let Some(core::result::Result::Ok(op)) = o.next().await {
            output.push_str(op.to_string().as_str());
            if output.len() >= LIMIT_BYTE {
                return Ok(format!("结果大于{}B,不予展示",LIMIT_BYTE));
            }
        }
    } else {
        return Err(anyhow!("Cannot attach io in the docker!"));
    }
    Ok(output)
}
