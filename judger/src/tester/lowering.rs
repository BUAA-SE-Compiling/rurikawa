//! Code for transforming test suite configs into something that [`crate::runner`]
//! can efficiently use.

use std::sync::Arc;

use bollard::Docker;
use futures::FutureExt;

use crate::runner::model::TestCase;

use super::model::{JudgeExecKind, JudgerPublicConfig};

pub struct RunnerPlan {}

pub async fn make_runner_plan(public_cfg: &JudgerPublicConfig) -> crate::runner::model::TestCase {
    todo!()
}
