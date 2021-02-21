//! Special Judge (SPJ) is a pluggable extension to the current judger, backed by
//! a JavaScript script.
//!
//! Read more about SPJ in `/docs/dev-manual/special-judger.md`

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context as AnyhowCtx;
use rquickjs::{Context, FromJs, Function, Promise, Runtime};
use tokio::{runtime::Handle, task::JoinHandle};

use super::{
    model::{JudgerPublicConfig, RawStep, TestCase},
    ProcessInfo,
};

pub const SPJ_INIT_FN: &str = "specialJudgeInit";
pub const SPJ_TRANSFORM_FN: &str = "specialJudgeTransformExec";
pub const SPJ_CASE_INIT_FN: &str = "specialJudgeCaseInit";
pub const SPJ_CASE_FN: &str = "specialJudgeCase";

pub const SPJ_MAX_MEMORY: usize = 50 * 1024 * 1024;

/// Run a function inside a context with promise-like return value
macro_rules! run_promise_like {
    ($ctx:expr,$name:expr,$args:expr,$map:expr) => {{
        let res = $ctx.with(move |ctx| {
            let globals = ctx.globals();
            if let Ok(f) = globals.get::<_, Function>($name) {
                let result = f.call::<_, rquickjs::Value>($args)?;
                Ok(extract_promise_like!(result))
            } else {
                Err(anyhow::anyhow!("{} is not a function!", $name))
            }
        })?;
        res.await.map($map)
    }};
}

/// Extract the future of a promise-like value
macro_rules! extract_promise_like {
    ($val:expr) => {
        if let Ok(p) = $val.get::<Promise<_>>() {
            futures::future::Either::Left(p)
        } else {
            futures::future::Either::Right(futures::future::ready($val.get()))
        }
    };
}

/// The execution environment of the special judge.
///
/// This environment handles both the runtime and the context.
pub struct SpjEnvironment {
    rt: Runtime,
    ctx: Context,
    features: SpjFeatures,
}

impl SpjEnvironment {
    /// Creates a new special judge environment.

    fn new() -> rquickjs::Result<SpjEnvironment> {
        // Create JS runtime
        let rt = rquickjs::Runtime::new()?;
        rt.set_memory_limit(SPJ_MAX_MEMORY);
        let ctx = Context::builder()
            .with::<rquickjs::intrinsic::Base>()
            .with::<rquickjs::intrinsic::BignumExt>()
            .with::<rquickjs::intrinsic::Date>()
            .with::<rquickjs::intrinsic::RegExp>()
            .with::<rquickjs::intrinsic::RegExpCompiler>()
            .with::<rquickjs::intrinsic::Json>()
            .with::<rquickjs::intrinsic::Promise>()
            .with::<rquickjs::intrinsic::TypedArrays>()
            .with::<rquickjs::intrinsic::Eval>()
            .build(&rt)?;
        Ok(SpjEnvironment {
            rt,
            ctx,
            features: Default::default(),
        })
    }

    pub fn with_readfile(&mut self, base_path: PathBuf) -> rquickjs::Result<()> {
        let readfile = binding::ReadFile(base_path);
        self.ctx.with(|ctx| ctx.globals().set("readFile", readfile))
    }

    pub fn with_console_env(&mut self, name: String) -> rquickjs::Result<()> {
        let console = binding::SpjConsole { ctx_name: name };
        self.ctx.with(|ctx| ctx.globals().set("console", console))
    }

    pub fn load_script(&mut self, script: &str) -> rquickjs::Result<()> {
        self.ctx.with(|ctx| -> Result<(), _> { ctx.eval(script) })?;
        self.features = self.detect_features();
        Ok(())
    }

    pub fn features(&self) -> &SpjFeatures {
        &self.features
    }

    /// Spawn all pending promises in the current async environment.
    ///
    /// This function is in fact **not** async, but making it async forces it to
    /// be run on some async environment, so [`tokio::runtime::Handle::current`].
    /// would run without problem.
    pub async fn spawn_futures(&self) -> JoinHandle<()> {
        self.rt.spawn_executor(TokioSpawner(Handle::current()))
    }

    /// Callback for initializing special judge
    pub async fn spj_global_init(&self, config: &JudgerPublicConfig) -> anyhow::Result<()> {
        run_promise_like!(self.ctx, SPJ_INIT_FN, (config,), |x| x).map_err(|e| e.into())
    }

    /// Callback for mapping exec
    pub async fn spj_map_exec(&self, config: &[RawStep]) -> anyhow::Result<Vec<RawStep>> {
        run_promise_like!(self.ctx, SPJ_TRANSFORM_FN, (config,), |x| x).map_err(|e| e.into())
    }

    /// Callback for case init
    pub async fn spj_case_init(
        &self,
        case: &TestCase,
        mappings: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        run_promise_like!(self.ctx, SPJ_CASE_INIT_FN, (case, mappings), |x| x).map_err(|e| e.into())
    }

    /// Callback for case judging
    pub async fn spj_case_judge(&self, results: &[ProcessInfo]) -> anyhow::Result<SpjResult> {
        run_promise_like!(self.ctx, SPJ_CASE_FN, (results,), |x| x).map_err(|e| e.into())
    }

    fn detect_features(&self) -> SpjFeatures {
        self.ctx.with(|ctx| {
            let globals = ctx.globals();
            let global_init = globals
                .get::<_, rquickjs::Value>(SPJ_INIT_FN)
                .map_or(false, |v| v.is_function());
            let transform_exec = globals
                .get::<_, rquickjs::Value>(SPJ_TRANSFORM_FN)
                .map_or(false, |v| v.is_function());
            let case_init = globals
                .get::<_, rquickjs::Value>(SPJ_CASE_INIT_FN)
                .map_or(false, |v| v.is_function());
            let case = globals
                .get::<_, rquickjs::Value>(SPJ_CASE_FN)
                .map_or(false, |v| v.is_function());
            SpjFeatures {
                global_init,
                transform_exec,
                case_init,
                case,
            }
        })
    }
}

/// Make a special judger using the given script path.
pub async fn make_spj(script_path: &Path) -> anyhow::Result<SpjEnvironment> {
    let mut spj = SpjEnvironment::new().context("when creating spj environment")?;
    let script = tokio::fs::read(script_path)
        .await
        .context("when reading spj input file")?;
    let script = String::from_utf8_lossy(&script);
    spj.load_script(&script)
        .context("when loading spj script")?;
    Ok(spj)
}

#[derive(Debug, FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct SpjResult {
    pub accepted: bool,
    pub score: Option<f64>,
    pub reason: Option<String>,
}

/// Represents enabled features of a SPJ instance. Methods of this
/// struct represents whether different functions are declared (and thus callable)
/// inside this instance.
///
/// See detail at `/docs/dev-manual/special-judger.md` in project root.
#[derive(Debug, Clone)]
pub struct SpjFeatures {
    /// Whether `specialJudgeInit` is declated as a function.
    global_init: bool,
    /// Whether `specialJudgeTransformExec` is declared as a function.
    transform_exec: bool,
    /// Whether `specialJudgeCaseInit` is declared as a function.
    case_init: bool,
    /// Whether `specialJudgeCase` is declared as a function.
    case: bool,
}

impl Default for SpjFeatures {
    fn default() -> Self {
        SpjFeatures {
            global_init: false,
            transform_exec: false,
            case_init: false,
            case: false,
        }
    }
}

impl SpjFeatures {
    /// Whether `specialJudgeInit` is declated as a function.
    pub fn global_init(&self) -> bool {
        self.global_init
    }

    /// Whether `specialJudgeTransformExec` is declared as a function.
    pub fn transform_exec(&self) -> bool {
        self.transform_exec
    }

    /// Whether `specialJudgeCaseInit` is declared as a function.
    pub fn case_init(&self) -> bool {
        self.case_init
    }

    /// Whether `specialJudgeCase` is declared as a function.
    pub fn case(&self) -> bool {
        self.case
    }
}

/// A task spawner for running tokio tasks.
struct TokioSpawner(Handle);

impl rquickjs::ExecutorSpawner for TokioSpawner {
    type JoinHandle = tokio::task::JoinHandle<()>;

    fn spawn_executor(self, task: rquickjs::Executor) -> Self::JoinHandle {
        self.0.spawn(task)
    }
}

/// Bindings into QuickJS instances
mod binding {
    use std::{path::PathBuf, sync::Arc};

    use rquickjs::{Async, Func, Function, IntoJs, MutFn};
    use tokio::io::AsyncReadExt;
    use tracing::info_span;

    use super::SPJ_MAX_MEMORY;

    /// Console to [`tracing`] logging adapter for QuickJS
    pub struct SpjConsole {
        pub ctx_name: String,
    }

    impl<'js> IntoJs<'js> for SpjConsole {
        fn into_js(
            self,
            ctx: rquickjs::Ctx<'js>,
        ) -> std::result::Result<rquickjs::Value<'js>, rquickjs::Error> {
            let span = info_span!(target: "qjs", "spj_console", ctx = %self.ctx_name);
            let obj = rquickjs::Object::new(ctx)?;

            obj.set("log", {
                let span = span.clone();
                Func::from(MutFn::from(move |s: String| {
                    let guard = span.enter();
                    tracing::info!("{}", s);
                    drop(guard);
                }))
            })
            .unwrap();

            obj.set("debug", {
                let span = span.clone();
                Func::from(MutFn::from(move |s: String| {
                    let guard = span.enter();
                    tracing::debug!("{}", s);
                    drop(guard);
                }))
            })
            .unwrap();

            obj.set("info", {
                let span = span.clone();
                Func::from(MutFn::from(move |s: String| {
                    let guard = span.enter();
                    tracing::info!("{}", s);
                    drop(guard);
                }))
            })
            .unwrap();

            obj.set("warn", {
                let span = span.clone();
                Func::from(MutFn::from(move |s: String| {
                    let guard = span.enter();
                    tracing::warn!("{}", s);
                    drop(guard);
                }))
            })
            .unwrap();

            obj.set("error", {
                Func::from(MutFn::from(move |s: String| {
                    let guard = span.enter();
                    tracing::error!("{}", s);
                    drop(guard);
                }))
            })
            .unwrap();

            Ok(obj.into())
        }
    }

    /// File reader for SPJ script.
    pub struct ReadFile(pub PathBuf);

    impl<'js> IntoJs<'js> for ReadFile {
        fn into_js(
            self,
            ctx: rquickjs::Ctx<'js>,
        ) -> std::result::Result<rquickjs::Value<'js>, rquickjs::Error> {
            let base_path = Arc::new(self.0);
            Function::new(
                ctx,
                Async(MutFn::from(move |path: (String,)| {
                    read_file(base_path.join(path.0))
                })),
            )
            .map(|f| f.into())
        }
    }

    /// Utility function for reading file
    pub async fn read_file(path: PathBuf) -> Option<String> {
        let mut file = tokio::fs::File::open(path).await.ok()?;
        let meta = file.metadata().await.ok()?;
        if meta.len() > SPJ_MAX_MEMORY as u64 {
            return None;
        }
        let mut result = String::new();
        file.read_to_string(&mut result).await.ok()?;
        Some(result)
    }
}

#[cfg(test)]
mod test {
    use crate::tester::model::{Bind, JudgerPublicConfig};
    use std::{collections::HashMap, path::PathBuf};

    #[tokio::test]
    async fn test_spj_async() {
        let script = r"
        function specialJudgeInit(config){
            return new Promise((res, rej)=>{
                console.log('hi')
                console.log('hi there')
                res()
            })
        }
        ";
        let mut spj = super::SpjEnvironment::new().unwrap();
        let config = JudgerPublicConfig {
            time_limit: None,
            memory_limit: None,
            name: "golem".into(),
            test_groups: HashMap::new(),
            vars: HashMap::new(),
            run: vec![],
            mapped_dir: Bind {
                from: PathBuf::from(r"../golem/src"),
                to: PathBuf::from(r"/golem/src"),
                readonly: false,
            },
            binds: Some(vec![]),
            special_judge_script: None,
        };

        spj.load_script(script).unwrap();
        spj.with_console_env("SPJ".into()).unwrap();
        spj.spawn_futures().await;
        eprintln!("start");
        spj.spj_global_init(&config).await.unwrap();
        eprintln!("end");
    }
}
