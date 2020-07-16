use super::util::{diff, strsignal};
use super::{ExecError, ExecErrorKind, JobConfig, JobFailure, OutputMismatch, ProcessInfo};
use std::time;
use subprocess::{CaptureData, Communicator, Exec, Pipeline, Popen, PopenError, Redirection};

type PopenResult<T> = Result<T, PopenError>;

#[derive(Debug)]
pub enum Capturable {
    Exec(Exec),
    Pipeline(Pipeline),
}

impl Capturable {
    pub fn capture(self) -> PopenResult<(String, CaptureData)> {
        match self {
            Capturable::Exec(e) => {
                let s = format!("{:?}", &e);
                Ok((s, e.capture()?))
            }
            Capturable::Pipeline(e) => {
                let s = format!("{:?}", &e);
                Ok((s, e.capture()?))
            }
        }
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

    pub fn capture(self) -> PopenResult<(String, CaptureData)> {
        if let Some(timeout) = self.timeout {
            todo!()
        } else {
            self.cmd.capture()
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

    pub fn run(self) -> Result<(), JobFailure> {
        let expected = self.expected.expect("Run Failed: Expected String not set");
        let mut output: Vec<ProcessInfo> = vec![];
        let steps_len = self.steps.len();
        for (i, step) in self.steps.into_iter().enumerate() {
            let captured = step
                .capture()
                .expect("Run Failed: Cannot launch subprocess");

            let info: ProcessInfo = captured.into();
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
                            strsignal(-code as i32)
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

    #[test]
    fn ok() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable::Exec(
            Exec::cmd("echo").arg("This does nothing."),
        )));
        t.add_step(Step::new(Capturable::Pipeline(
            Exec::cmd("echo").arg("Hello, world!") | Exec::cmd("awk").arg("{print $1}"),
        )));
        t.expected("Hello,\n");
        let res = t.run();
        assert!(matches!(dbg!(res), Ok(())));
    }

    #[test]
    fn error_code() {
        let mut t = Test::new();
        t.add_step(Step::new(Capturable::Exec(
            Exec::cmd("echo").arg("This does nothing."),
        )));
        t.add_step(Step::new(Capturable::Exec(Exec::shell(
            "echo 'Hello, world!' && false",
        ))));
        t.expected("Hello,\nworld!\n");
        let got = t.run();
        let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::ReturnCodeCheckFailed,
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Exec { echo \'This does nothing.\' }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: 1,
                    command: "Exec { sh -c \'echo \'\\\'\'Hello, world!\'\\\'\' && false\' }"
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
        t.add_step(Step::new(Capturable::Exec(
            Exec::cmd("echo").arg("This does nothing."),
        )));
        t.add_step(Step::new(Capturable::Exec(Exec::shell(
            // "ping www.bing.com & sleep 0.5; kill $!",
            "{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done",
        ))));
        t.expected("Hello,\nworld!\n");
        let got = t.run();
        let expected: Result<(), _> = Err(JobFailure::ExecError(ExecError {
            stage: 1,
            kind: ExecErrorKind::RuntimeError(
                "Runtime Error: Terminated: 15".into(),
            ),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Exec { echo \'This does nothing.\' }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: -15,
                    command: "Exec { sh -c \'{ sleep 0.1; kill $$; } & for (( i=0; i<4; i++ )) do echo $i; sleep 1; done\' }".into(),
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
        t.add_step(Step::new(Capturable::Exec(
            Exec::cmd("echo").arg("This does nothing."),
        )));
        t.add_step(Step::new(Capturable::Pipeline(
            Exec::cmd("echo").arg("Hello, world!") | Exec::cmd("awk").arg("{print $2}"),
        )));
        t.expected("Hello,\nworld!\n");
        let got = t.run();
        let expected: Result<(), _> = Err(JobFailure::OutputMismatch(OutputMismatch {
            diff: "+ Hello,\n  world!\n".into(),
            output: vec![
                ProcessInfo {
                    ret_code: 0,
                    command: "Exec { echo \'This does nothing.\' }".into(),
                    stdout: "This does nothing.\n".into(),
                    stderr: "".into(),
                },
                ProcessInfo {
                    ret_code: 0,
                    command: "Pipeline { echo \'Hello, world!\' | awk \'{print $2}\' }".into(),
                    stdout: "world!\n".into(),
                    stderr: "".into(),
                },
            ],
        }));
        assert_eq!(dbg!(got), expected);
    }
}
