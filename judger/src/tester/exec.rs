use super::diff::diff;
use super::{ExecError, ExecErrorKind, JobConfig, JobFailure, OutputMismatch, ProcessInfo};
use subprocess::{CaptureData, Exec, Pipeline, PopenError};

#[derive(Debug)]
pub enum Capturable {
    Exec(Exec),
    Pipeline(Pipeline),
}

impl Capturable {
    pub fn capture(self) -> Result<(String, CaptureData), PopenError> {
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

pub struct Test {
    steps: Vec<Capturable>,
    expected: Option<String>,
}

impl Test {
    pub fn new() -> Self {
        Test {
            steps: vec![],
            expected: None,
        }
    }

    pub fn add_step(&mut self, step: Capturable) -> &mut Self {
        self.steps.push(step);
        self
    }

    pub fn set_steps(&mut self, steps: Vec<Capturable>) -> &mut Self {
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
                        // TODO: Fix Error String
                        kind: ExecErrorKind::RuntimeError(format!(
                            "Runtime Error: Exit Code {}",
                            code
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
    fn test_echo() {
        let mut t = Test::new();
        t.add_step(Capturable::Exec(
            Exec::cmd("echo").arg("This does nothing."),
        ));
        t.add_step(Capturable::Pipeline(
            Exec::cmd("echo").arg("Hello, world!") | Exec::cmd("awk").arg("{print $1}"),
        ));
        t.expected("Hello,\n");
        let res = t.run();
        assert!(res.is_ok());
    }
}
