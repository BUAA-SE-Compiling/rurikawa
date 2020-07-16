use super::util::{diff, strsignal};
use super::{ExecError, ExecErrorKind, JobConfig, JobFailure, OutputMismatch, ProcessInfo};
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::Output;
use std::time;
use tokio::process::{self, Command};

type PopenResult<T> = Result<T, io::Error>;

#[macro_export]
macro_rules! command {
    ( $prog:expr, $( $arg:expr ),* ) => {
        {
            let mut cmd = tokio::process::Command::new($prog);
            $(
                cmd.arg($arg);
            )*
            cmd
        }
    };
}

#[macro_export]
macro_rules! shell {
    ( $script:expr ) => {{
        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c");
        cmd.arg($script);
        cmd
    }};
}

pub struct Capturable(Command);

impl Capturable {
    async fn capture(self) -> PopenResult<ProcessInfo> {
        let Self(mut cmd) = self;
        let cmd_str = format!("{:?}", cmd);
        let Output {
            status,
            stdout,
            stderr,
        } = cmd.output().await?;
        let ret_code = match (status.code(), status.signal()) {
            (Some(x), _) => x,
            (None, Some(x)) => -x,
            _ => unreachable!(),
        };
        Ok(ProcessInfo {
            command: cmd_str,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            ret_code,
        })
    }
}

pub struct Step {
    pub cmd: Capturable,
    pub timeout: Option<time::Duration>,
}

impl Step {
    pub fn new(cmd: Capturable) -> Self {
        Step { cmd, timeout: None }
    }

    pub fn timeout(mut self, timeout: time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub async fn capture(self) -> PopenResult<ProcessInfo> {
        if let Some(timeout) = self.timeout {
            let mres = tokio::time::timeout(timeout, self.cmd.capture()).await;
            if let Ok(res) = mres {
                res
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Popen capture timed out",
                ))
            }
        } else {
            self.cmd.capture().await
        }
    }
}

pub struct Test {
    steps: Vec<Step>,
    expected: Option<String>,
}

impl Test {
    pub fn new() -> Self {
        Test {
            steps: vec![],
            expected: None,
        }
    }

    pub fn add_step(&mut self, step: Step) -> &mut Self {
        self.steps.push(step);
        self
    }

    pub fn set_steps(&mut self, steps: Vec<Step>) -> &mut Self {
        self.steps = steps;
        self
    }

    pub fn expected(&mut self, expected: &str) -> &mut Self {
        self.expected = Some(expected.to_owned());
        self
    }

    pub async fn run(self) -> Result<(), JobFailure> {
        let expected = self.expected.expect("Run Failed: Expected String not set");
        let mut output: Vec<ProcessInfo> = vec![];
        let steps_len = self.steps.len();
        for (i, step) in self.steps.into_iter().enumerate() {
            let info = match step.capture().await {
                Ok(res) => res,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::TimedOut,
                        output,
                    }))
                }
                Err(_) => panic!("Run Failed: Cannot launch subprocess"),
            };

            output.push(info.clone());
            let code = info.ret_code;

            match () {
                _ if code > 0 => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::ReturnCodeCheckFailed,
                        output,
                    }));
                }
                _ if code < 0 => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::RuntimeError(format!(
                            "Runtime Error: {}",
                            strsignal(-code)
                        )),
                        output,
                    }));
                }
                _ => (),
            }

            // Special case for last step
            if i == steps_len - 1 {
                let got = info.stdout;
                if got != expected {
                    return Err(JobFailure::OutputMismatch(OutputMismatch {
                        diff: diff(&got, &expected),
                        output,
                    }));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn ok() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable(command!(
            "echo",
            "This does nothing."
        ))));
        t.add_step(Step::new(Capturable(shell!(
            "echo 'Hello, world!' | awk '{print $1}'"
        ))));
        t.expected("Hello,\n");
        let res = block_on(t.run());
        assert!(matches!(dbg!(res), Ok(())));
    }

    #[test]
    fn error_code() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable(command!(
            "echo",
            "This does nothing."
        ))));
        t.add_step(Step::new(Capturable(shell!(
            "echo 'Hello, world!' && false"
        ))));
        t.expected("Hello,\nworld!\n");
        let got = block_on(t.run());
        let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::ReturnCodeCheckFailed,
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Command { std: \"echo\" \"This does nothing.\", kill_on_drop: false }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: 1,
                    command: "Command { std: \"bash\" \"-c\" \"echo \\\'Hello, world!\\\' && false\", kill_on_drop: false }"
                        .into(),
                    stdout: "Hello, world!\n".into(),
                    stderr: "".into(),
                },
            ],
        }));
        assert_eq!(dbg!(got), expected);
    }

    #[test]
    fn signal() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable(command!(
            "echo",
            "This does nothing."
        ))));
        t.add_step(Step::new(Capturable(shell!(
            // "ping www.bing.com & sleep 0.5; kill $!",
            "{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done"
        ))));
        t.expected("Hello,\nworld!\n");
        let got = block_on(t.run());
        let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::RuntimeError(
                "Runtime Error: Terminated: 15".into(),
            ),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Command { std: \"echo\" \"This does nothing.\", kill_on_drop: false }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: -15,
                    command: "Command { std: \"bash\" \"-c\" \"{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done\", kill_on_drop: false }".into(),
                    stdout: "0\n".into(),
                    stderr: "".into(),
                },
            ],
        }));
        assert_eq!(dbg!(got), expected);
    }

    #[test]
    fn output_mismatch() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable(command!(
            "echo",
            "This does nothing."
        ))));
        t.add_step(Step::new(Capturable(shell!(
            "echo 'Hello, world!' | awk '{print $2}'"
        ))));
        t.expected("Hello,\nworld!\n");
        let got = block_on(t.run());
        let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
            diff: "+ Hello,\n  world!\n".into(),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Command { std: \"echo\" \"This does nothing.\", kill_on_drop: false }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: 0,
                    command: "Command { std: \"bash\" \"-c\" \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\", kill_on_drop: false }".into(),
                    stdout: "world!\n".into(),
                    stderr: "".into(),
                },
            ],
        }));
        assert_eq!(dbg!(got), expected);
    }

    #[test]
    fn output_timed_out() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable(command!(
            "echo",
            "This does nothing."
        ))));
        t.add_step(
            Step::new(Capturable(shell!("echo 0; sleep 3; echo 1")))
                .timeout(time::Duration::from_millis(100)),
        );
        t.expected("Hello,\nworld!\n");
        let got = block_on(t.run());
        let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::TimedOut,
            output: vec![ProcessInfo {
                ret_code: 0,
                command: "Command { std: \"echo\" \"This does nothing.\", kill_on_drop: false }"
                    .into(),
                stdout: "This does nothing.\n".into(),
                stderr: "".into(),
            }],
        }));
        assert_eq!(dbg!(got), expected);
    }
}
