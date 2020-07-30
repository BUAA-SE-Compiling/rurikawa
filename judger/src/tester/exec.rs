use super::utils::{diff, strsignal};
use super::{
    runner::CommandRunner, ExecError, ExecErrorKind, JobFailure, OutputMismatch, ProcessInfo,
    TestJob,
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
    pub fn new(cmd: Vec<String>) -> Self {
        Capturable(cmd)
    }

    async fn capture<R: CommandRunner + Send>(self, runner: &mut R) -> PopenResult<ProcessInfo> {
        let Self(cmd) = self;
        runner.run(&cmd).await
    }
}

/// One step in a `Test`.
pub struct Step {
    /// The command to be executed.
    pub cmd: Capturable,
    /// The timeout of the command.
    pub timeout: Option<time::Duration>,
}

impl Step {
    /// Make a new `Step` with no timeout.
    pub fn new(cmd: Capturable) -> Self {
        Step { cmd, timeout: None }
    }

    /// Set `timeout` for a `Step`.
    pub fn timeout(mut self, timeout: time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Make a new `Step` with a `timeout`.
    pub fn new_with_timeout(cmd: Capturable, timeout: Option<time::Duration>) -> Self {
        Step { cmd, timeout }
    }

    /// Run the `Step` and collect its info, considering the `timeout`.
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

/// A particular multi-`Step` test.
/// An I/O match test against `expected` is performed at the last `Step`.
pub struct Test {
    steps: Vec<Step>,
    /// The expected `stdout` content.
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
    use pretty_assertions::assert_eq as pretty_eq;
    use tokio_test::block_on;

    #[cfg(test)]
    mod tokio_runner {
        use super::*;
        use crate::tester::runner::TokioCommandRunner;

        #[test]
        fn ok() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $1}'"
                ))));
                t.expected("Hello,\n");
                let res = t.run(&mut TokioCommandRunner {}).await;
                assert!(matches!(dbg!(res), Ok(())));
            })
        }

        #[test]
        fn error_code() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' && false"
                ))));
                t.expected("Goodbye, world!");
                let got = t.run(&mut TokioCommandRunner {}).await;
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
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' && false\"]"
                                .into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn signal() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    // "ping www.bing.com & sleep 0.5; kill $!",
                    r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&mut TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::RuntimeError(
                        format!(
                            "Runtime Error: {}",
                            strsignal(15)
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
                            command: "[\"sh\", \"-c\", \"{ sleep 0.1; kill $$; } & i=0; while [ \\\"$i\\\" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done\"]".into(),
                            stdout: "0\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_mismatch() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $2}'"
                ))));
                t.expected("Hello,\nworld!");
                let got = t.run(&mut TokioCommandRunner {}).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n- ".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\"]".into(),
                            stdout: "world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_timed_out() {
            block_on(async {
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(
                    Step::new(Capturable::new(sh!("echo 0; sleep 3; echo 1")))
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&mut TokioCommandRunner {}).await;
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
                pretty_eq!(got, expected);
            })
        }
    }

    mod docker_runner {
        use super::*;
        use crate::tester::runner::DockerCommandRunner;
        use names::{Generator, Name};

        #[test]
        fn ok() {
            block_on(async {
                let mut names = Generator::with_naming(Name::Numbered);
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    &dbg!(names.next().unwrap()),
                    "alpine:latest",
                    None,
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $1}'"
                ))));
                t.expected("Hello,\n");
                let res = t.run(&mut runner).await;
                runner.kill().await;
                assert!(matches!(dbg!(res), Ok(())));
            });
        }

        #[test]
        fn error_code() {
            block_on(async {
                let mut names = Generator::with_naming(Name::Numbered);
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    &dbg!(names.next().unwrap()),
                    "alpine:latest",
                    None,
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' && false"
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&mut runner).await;
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
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' && false\"]"
                                .into(),
                            stdout: "Hello, world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                runner.kill().await;
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn signal() {
            block_on(async {
                let mut names = Generator::with_naming(Name::Numbered);
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    &dbg!(names.next().unwrap()),
                    "alpine:latest",
                    None,
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    // "ping www.bing.com & sleep 0.5; kill $!",
                    r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
                ))));
                t.expected("Hello,\nworld!\n");
                let got = t.run(&mut runner).await;
                let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
                    stage: 1,
                    kind: ExecErrorKind::RuntimeError(
                        format!(
                            "Runtime Error: {}",      
                            strsignal(15)
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
                            command: "[\"sh\", \"-c\", \"{ sleep 0.1; kill $$; } & i=0; while [ \\\"$i\\\" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done\"]".into(),
                            stdout: "0\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                runner.kill().await;
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_mismatch() {
            block_on(async {
                let mut names = Generator::with_naming(Name::Numbered);
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    &dbg!(names.next().unwrap()),
                    "alpine:latest",
                    None,
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(Step::new(Capturable::new(sh!(
                    "echo 'Hello, world!' | awk '{print $2}'"
                ))));
                t.expected("Hello,\nworld!");
                let got = t.run(&mut runner).await;
                let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                    diff: "+ Hello,\n  world!\n- ".into(),
                    output: vec![
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"echo\", \"This does nothing.\"]".into(),
                            stdout: "This does nothing.\n".into(),
                            stderr: "".into(),
                        },
                        ProcessInfo {
                            ret_code: 0,
                            command: "[\"sh\", \"-c\", \"echo \\\'Hello, world!\\\' | awk \\\'{print $2}\\\'\"]".into(),
                            stdout: "world!\n".into(),
                            stderr: "".into(),
                        },
                    ],
                }));
                runner.kill().await;
                pretty_eq!(got, expected);
            })
        }

        #[test]
        fn output_timed_out() {
            block_on(async {
                let mut names = Generator::with_naming(Name::Numbered);
                let mut runner = DockerCommandRunner::new(
                    bollard::Docker::connect_with_unix_defaults().unwrap(),
                    &dbg!(names.next().unwrap()),
                    "alpine:latest",
                    None,
                )
                .await;
                let mut t = Test::new();
                t.add_step(Step::new(Capturable::new(command!(
                    "echo",
                    "This does nothing."
                ))));
                t.add_step(
                    Step::new(Capturable::new(sh!("echo 0; sleep 3; echo 1")))
                        .timeout(time::Duration::from_millis(100)),
                );
                t.expected("Hello,\nworld!\n");
                let got = t.run(&mut runner).await;
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
                runner.kill().await;
                pretty_eq!(got, expected);
            })
        }
    }
}
