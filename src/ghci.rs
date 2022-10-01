use anyhow::anyhow;
use std::io::Write;
use std::process::*;

pub const LIMIT_BYTE: usize = 1000;
const TIME_OUT: u8 = 3;

pub fn execute(expr: String) -> anyhow::Result<Output> {
    let mut command = Command::new("timeout")
        .args(format!("-s 5 {} ghci", TIME_OUT).split_whitespace())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

/// The result of command execution.
pub enum ExecutionResult {
    /// The string read from `stdout` and `stderr`
    Output { stdout: String, stderr: String },
    /// Timeout during execution
    Timeout,
    /// Output length exceeded the limit
    LengthExceeded,
    /// Other errors that happened when handling the output
    OtherError(anyhow::Error),
}

/// Handle the output produced from the command executed.
pub fn process_output(output: Output) -> ExecutionResult {
    if output.status.success() {
        let outputs = try {
            (
                String::from_utf8(output.stdout)?,
                String::from_utf8(output.stderr)?,
            )
        };
        let (stdout, stderr) = match outputs {
            Ok(r) => r,
            Err(e) => return ExecutionResult::OtherError(e),
        };

        if stdout.len() < LIMIT_BYTE {
            ExecutionResult::Output {
                stdout: process_stdout_string(&stdout),
                stderr,
            }
        } else {
            ExecutionResult::LengthExceeded
        }
    } else {
        ExecutionResult::Timeout
    }
}

/// Trims GHCi greetings and leaving messages from GHCi's output.
///
/// Note: needs a proper `.ghci` to be loaded in order for this to work. Example:
/// ```
/// :set prompt ""
/// :set prompt-cont ""
/// ```
fn process_stdout_string(output: &str) -> String {
    output
        .trim()
        .strip_suffix("Leaving GHCi.")
        .map(|o| o.lines().skip(2).intersperse("\n").collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_stdout() {
        let output = "GHCi, version 8.10.7: https://www.haskell.org/ghc/  :? for help
Loaded GHCi configuration from /foo/bar
1
Const 1 :: Num c => Const c a
Leaving GHCi.
";
        let expected = "1
Const 1 :: Num c => Const c a";
        assert_eq!(process_stdout_string(output), expected);

        let output = "GHCi, version 9.2.4: https://www.haskell.org/ghc/  :? for help
Loaded GHCi configuration from /foo/bar
outputLeaving GHCi.
";
        let expected = "output";
        assert_eq!(process_stdout_string(output), expected);
    }
}
