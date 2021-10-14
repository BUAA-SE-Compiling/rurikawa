//! Concrete implementation on running tests.
//!
//! This module is not responsible for interpreting test suites. See [`crate::tester`]
//! for corresponding code.

use tokio::sync::mpsc::Sender;

use crate::tester::ProcessInfo;

pub mod exec;
pub mod image;
pub mod model;
mod util;

pub async fn run_test_case(
    exec: &model::TestCase,
    opt: &model::CommandRunOptions,
    sink: Sender<ProcessInfo>,
) -> anyhow::Result<()> {
    for group in &exec.commands {
        run_exec_group(group, opt, sink.clone()).await?;
    }
    Ok(())
}

pub async fn run_exec_group(
    group: &model::ExecGroup,
    opt: &model::CommandRunOptions,
    sink: Sender<ProcessInfo>,
) -> anyhow::Result<()> {
    for exec in &group.steps {
        let run_res = match group
            .run_in
            .run(
                &exec.run,
                &mut exec.env.iter().map(|(k, v)| (k.as_ref(), v.as_ref())),
                opt,
            )
            .await
        {
            Ok(o) => o,
            Err(e) => {
                return Err(e);
            }
        };

        if run_res.ret_code == 0 {
            sink.send(run_res).await?;
            break;
        }

        sink.send(run_res).await?;
    }
    Ok(())
}
