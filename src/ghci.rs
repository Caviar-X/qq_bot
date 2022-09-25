use anyhow::*;
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions};
use bollard::exec::*;
use std::string::String;
use bollard::*;
use std::collections::HashMap;
use tokio::task::spawn;
use tokio::io::{AsyncWrite,AsyncWriteExt};
const CONTAINER_NAME: &'static str = "ghci_container";
const IMAGE_NAME: &'static str = "archlinux";
/// 5 seconds
pub const STOP_TIMEOUT: i64 = 5;
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
pub async fn create_container(
    name: String,
    image_name: String,
    stop_timeout: i64,
) -> Result<String> {
    Ok(Docker::connect_with_local_defaults()?
        .create_container(
            Some(CreateContainerOptions::<String> { name }),
            Config::<String> {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                open_stdin: Some(true),
                image: Some(image_name),
                stop_timeout: Some(stop_timeout),
                ..Default::default()
            },
        )
        .await?
        .id)
}
pub async fn execute(expr: String) -> Result<String> {
    let docker = Docker::connect_with_local_defaults()?;
    let mut id = String::new();
    if !is_container_running(get_id_by_name(CONTAINER_NAME.into()).await?).await? {
        id = create_container(CONTAINER_NAME.into(), IMAGE_NAME.into(), STOP_TIMEOUT).await?;
    }
    id = get_id_by_name(CONTAINER_NAME.into()).await?;
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
    if let StartExecResults::Attached { input: mut i,mut output}  = docker.start_exec(&exec,None).await? {
        spawn(async move {
            i.write_all(expr.as_bytes()).await.ok();
        });
    } else {
        return Err(anyhow!("Cannot attach io in the docker!"));
    }
    Ok(String::new())

}
