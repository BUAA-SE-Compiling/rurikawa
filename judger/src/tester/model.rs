use anyhow::Result;
use bollard::models::Mount;
use path_absolutize::Absolutize;
use rquickjs::{FromJs, IntoJsByRef};
use serde::{self, Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, string::String};
use std::{path::Path, str::FromStr};

/// A Host-to-container volume binding for the container.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bind {
    /// Absolute/Relative `from` path (in the host machine).
    pub from: PathBuf,
    /// Absolute `to` path (in the container).
    pub to: PathBuf,
    // Note: Removed readonly option here, since all binds should be readonly
    // for security reasons.
}

impl Bind {
    pub fn canonical_from(&mut self, base_dir: &Path) {
        let mut from_base = base_dir.to_owned();
        from_base.push(&self.from);
        self.from = from_base.absolutize().unwrap().into_owned();
    }

    pub fn to_mount(&self) -> Mount {
        Mount {
            target: Some(self.to.display().to_string()),
            source: Some(self.from.display().to_string()),
            typ: Some(bollard::models::MountTypeEnum::BIND),
            // all binds should be readonly for security reasons.
            read_only: Some(true),
            ..Default::default()
        }
    }
}

pub fn path_canonical_from(path: &Path, base_dir: &Path) -> PathBuf {
    let mut from_base = base_dir.to_owned();
    from_base.push(path);
    from_base.absolutize().unwrap().into_owned()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "source")]
#[serde(rename_all = "camelCase")]
pub enum Image {
    /// An existing image.
    Image { tag: String },
    /// An image to be built with a Dockerfile.
    Dockerfile {
        /// Name to be assigned to the image.
        tag: String,
        /// Path of the context directory, relative to the context directory.
        path: PathBuf,
        /// Path of the dockerfile itself, relative to the context directory.
        /// Leaving this value to None means using the default dockerfile: `path/Dockerfile`.
        file: Option<PathBuf>,
    },
}

/// The definition of a test case
#[derive(Serialize, Debug, Clone, IntoJsByRef)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseDefinition {
    pub name: String,
    pub should_fail: bool,
    pub has_out: bool,

    /// Baseline score for this test case
    #[serde(default = "default_base_score")]
    pub base_score: f64,
}

impl FromStr for TestCaseDefinition {
    type Err = crate::util::Void;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TestCaseDefinition {
            name: s.to_owned(),
            should_fail: false,
            has_out: true,
            base_score: 1.0,
        })
    }
}

/// Judger's public config, specific to a paticular repository,
/// Maintained by the owner of the project to be tested.
#[derive(Serialize, Deserialize, Debug, Clone, IntoJsByRef)]
#[serde(rename_all = "camelCase")]
#[quickjs(rename_all = "camelCase")]
pub struct JudgerPublicConfig {
    pub time_limit: Option<i32>,
    pub memory_limit: Option<i32>,
    pub name: String,
    pub test_groups: HashMap<String, Vec<TestCaseDefinition>>,

    /// Variables and extensions of test files
    /// (`$src`, `$bin`, `$stdin`, `$stdout`, etc...).
    /// For example: `"$src" => "go"`.
    pub vars: HashMap<String, String>,

    /// Sequence of commands necessary to perform an IO check.
    pub run: Vec<String>,

    /// The path of test root directory to be mapped inside test container
    #[quickjs(skip)]
    pub mapped_dir: Bind,

    /// `host-src:container-dest` volume bindings for the container. **Binds are
    /// always readonly for security reasons.**
    /// For details see [here](https://docs.rs/bollard/0.7.2/bollard/service/struct.HostConfig.html#structfield.binds).
    #[quickjs(skip)]
    pub binds: Option<Vec<Bind>>,

    /// Path to the special judger script.
    ///
    /// The special judger script should be a valid JS script with specified
    /// functions inside global scope.
    pub special_judge_script: Option<String>,
}

/// A raw step for usage in spj scripts
#[derive(IntoJsByRef, FromJs)]
#[quickjs(rename_all = "camelCase")]
pub struct RawStep {
    pub command: String,
    pub is_user_command: bool,
}

/// Judger's private config, specific to a host machine.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JudgerPrivateConfig {
    /// Directory of test sources files (including `stdin` and `stdout` files)
    /// outside the container.
    pub test_root_dir: PathBuf,
    /// Directory of test sources files inside the container.
    pub mapped_test_root_dir: PathBuf,
}

/// The public representation of a test.
#[derive(Serialize, Deserialize, Debug, Clone, IntoJsByRef)]
#[quickjs(rename_all = "camelCase")]
pub struct TestCase {
    /// File name of the test case.
    pub name: String,
    /// Expected `stdout` of the last command.
    pub expected_out: Option<String>,
    /// Should this test case fail
    pub should_fail: bool,

    /// Baseline score for this test case
    #[serde(default = "default_base_score")]
    pub base_score: f64,
}

fn default_base_score() -> f64 {
    1.0
}

/// Initialization options for `Testsuite`.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestSuiteOptions {
    /// File names of tests.
    pub tests: Vec<String>,
    /// Time limit of a step, in seconds.
    pub time_limit: Option<usize>,
    // TODO: Use this field.
    /// Memory limit of the contrainer, in bytes.
    pub mem_limit: Option<usize>,
    /// If the image needs to be built before run.
    pub build_image: bool,
    /// If the image needs to be removed after run.
    pub remove_image: bool,
}

impl Default for TestSuiteOptions {
    fn default() -> Self {
        TestSuiteOptions {
            tests: vec![],
            time_limit: None,
            mem_limit: None,
            build_image: false,
            remove_image: false,
        }
    }
}

mod de {
    use super::TestCaseDefinition;
    use serde::{
        de::Deserializer,
        de::MapAccess,
        de::{self, Visitor},
        Deserialize,
    };
    use std::str::FromStr;

    macro_rules! set_field {
        ($field:expr, $map:expr) => {{
            if $field.is_some() {
                return Err(de::Error::duplicate_field(stringify!(field)));
            }
            $field = Some($map.next_value()?);
        }};
    }

    macro_rules! check_field {
        ($field:expr) => {
            $field.ok_or_else(|| de::Error::missing_field(stringify!($field)))?
        };
    }

    #[derive(Deserialize)]
    #[serde(field_identifier, rename_all = "camelCase")]
    enum TestCaseFields {
        Name,
        ShouldFail,
        HasOut,
        BaseScore,
    }

    struct TestCaseVisitor;

    impl<'de> Visitor<'de> for TestCaseVisitor {
        type Value = TestCaseDefinition;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "string or test case definition")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            TestCaseDefinition::from_str(v).map_err(|_| de::Error::custom("never"))
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut name = None;
            let mut should_fail = None;
            let mut has_out = None;
            let mut base_score = None;

            while let Some(key) = map.next_key::<TestCaseFields>()? {
                match key {
                    TestCaseFields::Name => set_field!(name, map),
                    TestCaseFields::ShouldFail => set_field!(should_fail, map),
                    TestCaseFields::HasOut => set_field!(has_out, map),
                    TestCaseFields::BaseScore => set_field!(base_score, map),
                }
            }

            let name = check_field!(name);
            let should_fail = should_fail.unwrap_or(false);
            let has_out = has_out.unwrap_or(true);
            let base_score = base_score.unwrap_or(1.0);

            Ok(TestCaseDefinition {
                name,
                should_fail,
                has_out,
                base_score,
            })
        }
    }

    impl<'de> Deserialize<'de> for TestCaseDefinition {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(TestCaseVisitor)
        }
    }
}
