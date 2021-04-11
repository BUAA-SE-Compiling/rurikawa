#![cfg(test)]
use super::*;
use pretty_assertions::assert_eq as pretty_eq;
use tokio_test::block_on;

#[cfg(unix)]
mod tokio_runner {
    use super::*;
    use crate::tester::runner::TokioCommandRunner;

    #[test]
    fn ok() {
        block_on(async {
            let mut t = Test::new();
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' | awk '{print $1}'"),
                true,
            ));
            t.expected("Hello,\n");
            let res = t.run(&TokioCommandRunner {}, &HashMap::new(), None).await;
            assert!(matches!(dbg!(res), Ok(_)));
        })
    }

    #[test]
    fn error_code() {
        block_on(async {
            let mut t = Test::new();
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' && false"),
                true,
            ));
            t.expected("Goodbye, world!");
            let got = t.run(&TokioCommandRunner {}, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::ReturnCodeCheckFailed,
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        command: "echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                        is_user_command: true,
                    },
                    ProcessInfo {
                        ret_code: 1,
                        command: "echo 'Hello, world!' && false".into(),
                        stdout: "Hello, world!\n".into(),
                        stderr: "".into(),
                        is_user_command: true,
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
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(Capturable::new(
                // Kill a running task
                r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
            ),true));
            t.expected("Hello,\nworld!\n");
            let got = t.run(&TokioCommandRunner {}, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::RuntimeError(
                    format!(
                        "Runtime Error: {} (signal 15)",
                        strsignal(15).unwrap()
                    )
                ),
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command:true,
                        command: r"echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    },
                    ProcessInfo {
                        ret_code: -15,
                        is_user_command:true,
                        command:r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#.into(),
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
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' | awk '{print $2}'"),
                true,
            ));
            t.expected("Hello,\nworld!");
            let got = t.run(&TokioCommandRunner {}, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                diff: "+ Hello,\n  world!\n".into(),
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: r"echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    },
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: "echo 'Hello, world!' | awk '{print $2}'".into(),
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
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(
                Step::new(Capturable::new("echo 0; sleep 3; echo 1"), true)
                    .set_timeout(time::Duration::from_millis(100)),
            );
            t.expected("Hello,\nworld!\n");
            let got = t.run(&TokioCommandRunner {}, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::TimedOut,
                output: vec![ProcessInfo {
                    ret_code: 0,
                    is_user_command: true,
                    command: r"echo 'This does nothing.'".into(),
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
    use crate::tester::runner::{DockerCommandRunner, DockerCommandRunnerOptions};

    fn docker_run<F, O>(f: F)
    where
        F: FnOnce(DockerCommandRunner, Test) -> O,
        O: futures::Future<Output = DockerCommandRunner>,
    {
        block_on(async {
            let runner = DockerCommandRunner::try_new(
                bollard::Docker::connect_with_local_defaults().unwrap(),
                Image::Prebuilt {
                    tag: "alpine:latest".to_owned(),
                },
                DockerCommandRunnerOptions {
                    build_image: true,
                    ..Default::default()
                },
                Option::<BuildResultChannel>::None,
            )
            .await
            .unwrap();
            let t = Test::new();
            f(runner, t).await.kill().await;
        });
    }

    #[test]
    fn ok() {
        docker_run(|runner, mut t| async {
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' | awk '{print $1}'"),
                true,
            ));
            t.expected("Hello,\n");
            let res = t.run(&runner, &HashMap::new(), None).await;
            // Any Ok(_) represents accepted, just with different score.
            assert!(matches!(dbg!(res), Ok(_)));
            runner
        });
    }

    #[test]
    fn error_code() {
        docker_run(|runner, mut t| async {
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' && false"),
                true,
            ));
            t.expected("Hello,\nworld!\n");
            let got = t.run(&runner, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::ReturnCodeCheckFailed,
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        command: "echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                        is_user_command: true,
                    },
                    ProcessInfo {
                        ret_code: 1,
                        command: "echo 'Hello, world!' && false".into(),
                        stdout: "Hello, world!\n".into(),
                        stderr: "".into(),
                        is_user_command: true,
                    },
                ],
            }));
            pretty_eq!(got, expected);
            runner
        })
    }

    #[test]
    fn signal() {
        docker_run(|runner, mut t| async {
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(Capturable::new(
                // Kill a running task
                r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#
            ),true));
            t.expected("Hello,\nworld!\n");
            let got = t.run(&runner, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::RuntimeError(
                    if cfg!(unix){ 
                        format!(
                            "Runtime Error: {} (signal 15)",
                            strsignal(15).unwrap()
                        )
                    }else{
                        "Runtime Error: signal 15".into()
                    }
                ),
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command:true,
                        command: r"echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    },
                    ProcessInfo {
                        ret_code: -15,
                        is_user_command:true,
                        command:r#"{ sleep 0.1; kill $$; } & i=0; while [ "$i" -lt 4 ]; do echo $i; sleep 1; i=$(( i + 1 )); done"#.into(),
                        stdout: "0\n".into(),
                        stderr: "".into(),
                    },
                ],
            }));
            pretty_eq!(got, expected);
            runner
        })
    }

    #[test]
    fn output_mismatch() {
        docker_run(|runner, mut t| async {
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(Step::new(
                Capturable::new("echo 'Hello, world!' | awk '{print $2}'"),
                true,
            ));
            t.expected("Hello,\nworld!");
            let got = t.run(&runner, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::OutputMismatch(OutputMismatch {
                diff: "+ Hello,\n  world!\n".into(),
                output: vec![
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: r"echo 'This does nothing.'".into(),
                        stdout: "This does nothing.\n".into(),
                        stderr: "".into(),
                    },
                    ProcessInfo {
                        ret_code: 0,
                        is_user_command: true,
                        command: "echo 'Hello, world!' | awk '{print $2}'".into(),
                        stdout: "world!\n".into(),
                        stderr: "".into(),
                    },
                ],
            }));
            pretty_eq!(got, expected);
            runner
        })
    }

    #[test]
    fn output_timed_out() {
        docker_run(|runner, mut t| async {
            t.add_step(Step::new(
                Capturable::new(r"echo 'This does nothing.'"),
                true,
            ));
            t.add_step(
                Step::new(Capturable::new("echo 0; sleep 3; echo 1"), true)
                    .set_timeout(time::Duration::from_millis(100)),
            );
            t.expected("Hello,\nworld!\n");
            let got = t.run(&runner, &HashMap::new(), None).await;
            let expected: Result<f64, _> = Err(JobFailure::ExecError(ExecError {
                stage: 1,
                kind: ExecErrorKind::TimedOut,
                output: vec![ProcessInfo {
                    ret_code: 0,
                    is_user_command: true,
                    command: r"echo 'This does nothing.'".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                }],
            }));
            pretty_eq!(got, expected);
            runner
        })
    }
}
