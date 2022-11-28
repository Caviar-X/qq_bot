use proc_qq::re_exports::ricq_core::msg::elem::RQElem;
use proc_qq::*;
#[event]
async fn heartbeat(event: &GroupMessageEvent) -> Result<bool> {
    let mut at = 0;
    let chain = &event.message_chain().0;
    for i in chain {
        if let RQElem::At(a) = i.clone().into() {
            at = a.target;
        }
    }
    if at == 0 {
        return Ok(true);
    } else if at == event.bot_uin().await && event.message_content().contains("还能说话吗") {
        event
            .send_message_to_source("能说话".parse_message_chain())
            .await?;
        return Ok(true);
    }
    Ok(false)
}
pub fn module() -> Module {
    module!("test", "t", heartbeat)
}
