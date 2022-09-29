use anyhow::*;
use std::io::Write;
use std::process::*;
const CONTAINER_NAME: &'static str = "ghci_con";
pub const LIMIT_BYTE: usize = 1000;
const TIME_OUT: u8 = 3;
pub fn execute(expr: String) -> Result<Output> {
    let mut command = Command::new("docker")
        .args(
            format!("exec -i {} timeout -s 5 {} ghci", CONTAINER_NAME, TIME_OUT)
                .split_whitespace()
                .collect::<Vec<&str>>(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let mut stdin = command
        .stdin
        .take()
        .ok_or_else(|| anyhow!("cannot open the stdin"))?;
    std::thread::spawn(move || {
        stdin.write_all(expr.as_bytes()).unwrap();
    });
    Ok(command.wait_with_output()?)
}
