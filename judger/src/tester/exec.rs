use super::util::{diff, strsignal};
use super::{
    runner::CommandRunner, ExecError, ExecErrorKind, JobConfig, JobFailure, OutputMismatch,
    ProcessInfo,
};
use crate::prelude::*;
use std::io;
use std::time;

#[macro_export]
macro_rules! command {
    ( $prog:expr, $( $arg:expr ),* ) => {
        {
            let mut cmd = vec![$prog.to_string()];
            $(
                cmd.push($arg.to_string());
            )*
            cmd
        }
    };
}

#[macro_export]
macro_rules! bash {
    ( $script:expr ) => {{
        let mut cmd = vec!["bash".to_owned(), "-c".to_owned()];
        cmd.push($script.to_string());
        cmd
    }};
}

#[macro_export]
macro_rules! sh {
    ( $script:expr ) => {{
        let mut cmd = vec!["sh".to_owned(), "-c".to_owned()];
        cmd.push($script.to_string());
        cmd
    }};
}

pub struct Capturable(Vec<String>);

impl Capturable {
    async fn capture<R: CommandRunner + Send>(self, runner: &mut R) -> PopenResult<ProcessInfo> {
        let Self(cmd) = self;
        runner.run(&cmd).await
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

    pub async fn capture<R>(self, runner: &mut R) -> PopenResult<ProcessInfo>
    where
        R: CommandRunner + Send,
    {
        if let Some(timeout) = self.timeout {
            let mres = tokio::time::timeout(timeout, self.cmd.capture(runner)).await;
            if let Ok(res) = mres {
                res
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Popen capture timed out",
                ))
            }
        } else {
            self.cmd.capture(runner).await
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

    pub async fn run<R>(self, runner: &mut R) -> Result<(), JobFailure>
    where
        R: CommandRunner + Send,
    {
        let expected = self.expected.expect("Run Failed: Expected String not set");
        let mut output: Vec<ProcessInfo> = vec![];
        let steps_len = self.steps.len();
        for (i, step) in self.steps.into_iter().enumerate() {
            let info = match step.capture(runner).await {
                Ok(res) => res,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => {
                    return Err(JobFailure::ExecError(ExecError {
                        stage: i,
                        kind: ExecErrorKind::TimedOut,
                        output,
                    }))
                }
                Err(e) => panic!("Run Failed: Cannot launch subprocess, {}", e),
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

    #[cfg(test)]
    mod tokio_runner {
        use super::*;
        use crate::tester::runner::TokioCommandRunner;

        #[test]
        fn ok() {
            let mut t = Test::new();
            t.add_step(Step::new(Capturable(command!(
                "echo",
                "This does nothing."
            ))));
            t.add_step(Step::new(Capturable(bash!(
                "echo 'Hello, world!' | awk '{print $1}'"
            ))));
            t.expected("Hello,\n");
            let res = block_on(t.run(&mut TokioCommandRunner {}));
            assert!(matches!(dbg!(res), Ok(())));
        }

        #[test]
        fn error_code() {
            let mut t = Test::new();
            t.add_step(Step::new(Capturable(command!(
                "echo",
                "This does nothing."
            ))));
            t.add_step(Step::new(Capturable(bash!(
                "echo 'Hello, world!' && false"
            ))));
            t.expected("Hello,\nworld!\n");
            let got = block_on(t.run(&mut TokioCommandRunner {}));
            let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::ReturnCodeCheckFailed,
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        command: "[\"echo\", \"This does nothing.\"]".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    },
                    ProcessInfo {
                        ret_code: 1,
                        command: "[\"bash\", \"-c\", \"echo \\\'Hello, world!\\\' && false\"]"
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
            t.add_step(Step::new(Capturable(bash!(
                // "ping www.bing.com & sleep 0.5; kill $!",
                "{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done"
            ))));
            t.expected("Hello,\nworld!\n");
            let got = block_on(t.run(&mut TokioCommandRunner {}));
            let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::RuntimeError(
                format!(

                    "Runtime Error: {}",      strsignal(15)
                )
            ),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "[\"echo\", \"This does nothing.\"]".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: -15,
                    command: "[\"bash\", \"-c\", \"{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done\"]".into(),
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
            t.add_step(Step::new(Capturable(bash!(
                "echo 'Hello, world!' | awk '{print $2}'"
            ))));
            t.expected("Hello,\nworld!\n");
            let got = block_on(t.run(&mut TokioCommandRunner {}));
            let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
            diff: "+ Hello,\n  world!\n".into(),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "[\"echo\", \"This does nothing.\"]".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: 0,
                    command: "[\"bash\", \"-c\", \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\"]".into(),
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
                Step::new(Capturable(bash!("echo 0; sleep 3; echo 1")))
                    .timeout(time::Duration::from_millis(100)),
            );
            t.expected("Hello,\nworld!\n");
            let got = block_on(t.run(&mut TokioCommandRunner {}));
            let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::TimedOut,
                output: vec![ProcessInfo {
                    ret_code: 0,
                    command: "[\"echo\", \"This does nothing.\"]".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                }],
            }));
            assert_eq!(dbg!(got), expected);
        }
    }

    mod docker_runner {
        use super::*;
        use crate::tester::runner::DockerCommandRunner;

        #[test]
        fn ok() {
            block_on(async {
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    "rurikawa_tester",
                    "alpine:latest",
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable(sh!(
                    "echo 'Hello, world!' | awk '{print $1}'"
                ))));
                t.expected("Hello,\n");
                let res = t.run(&mut runner).await;
                runner.kill().await;
                assert!(matches!(dbg!(res), Ok(())));
            });
        }
    }
}
