#![cfg(test)]
use super::*;
use tokio_test::block_on;

#[test]
fn golem_no_volume() -> Result<()> {
    block_on(async {
        let image_name = "golem_no_volume";
        // Repo directory in the host FS.
        let host_repo_dir = PathBuf::from(r"../golem");

        let mut ts = TestSuite::from_config(
            Image::Dockerfile {
                tag: image_name.to_owned(),
                path: host_repo_dir,
                file: None,
            },
            &std::env::current_dir().unwrap(),
            JudgerPrivateConfig {
                test_root_dir: PathBuf::from(r"../golem/src"),
                mapped_test_root_dir: PathBuf::from(r"/golem/src"),
            },
            JudgerPublicConfig {
                time_limit: None,
                memory_limit: None,
                name: "golem_no_volume".into(),
                test_groups: {
                    [(
                        "default".to_owned(),
                        vec![TestCaseDefinition {
                            name: "succ".into(),
                            should_fail: false,
                            has_out: true,
                            base_score: 1.0,
                        }],
                    )]
                    .iter()
                    .cloned()
                    .collect()
                },
                vars: [
                    ("$src", "py"),
                    ("$bin", "pyc"),
                    ("$stdin", "in"),
                    ("$stdout", "out"),
                ]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
                run: ["cat $stdin | python ./golem.py $bin"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),

                mapped_dir: Bind {
                    from: PathBuf::from(r"../golem/src"),
                    to: PathBuf::from(r"../golem/src"),
                    readonly: false,
                },
                binds: None,
                special_judge_script: None,
            },
            &JudgeTomlTestConfig {
                // TODO: Refine interface
                image: Image::Image { tag: "".into() },
                build: None,
                run: vec!["python ./golemc.py $src -o $bin".into()],
            },
            TestSuiteOptions {
                tests: ["succ"].iter().map(|s| s.to_string()).collect(),
                time_limit: None,
                mem_limit: None,
                build_image: true,
                remove_image: true,
            },
        )
        .await?;

        let instance = bollard::Docker::connect_with_local_defaults().unwrap();
        ts.run(
            instance,
            std::env::current_dir().unwrap(),
            None,
            None,
            None,
            Default::default(),
        )
        .await?;
        Ok(())
    })
}

#[test]
fn golem_with_volume() -> Result<()> {
    block_on(async {
        let image_name = "golem";
        // Repo directory in the host FS.
        let host_repo_dir = PathBuf::from(r"../golem");

        let mut ts = TestSuite::from_config(
            Image::Dockerfile {
                tag: image_name.to_owned(),
                path: host_repo_dir, // public: c# gives repo remote, rust clone and unzip
                file: None,
            },
            &std::env::current_dir().unwrap(),
            JudgerPrivateConfig {
                test_root_dir: PathBuf::from(r"../golem/src"),
                mapped_test_root_dir: PathBuf::from(r"/golem/src"),
            },
            JudgerPublicConfig {
                time_limit: None,
                memory_limit: None,
                name: "golem".into(),
                test_groups: {
                    [(
                        "default".to_owned(),
                        vec![TestCaseDefinition {
                            name: "succ".into(),
                            should_fail: false,
                            has_out: true,
                            base_score: 1.0,
                        }],
                    )]
                    .iter()
                    .cloned()
                    .collect()
                },
                vars: [
                    ("$src", "py"),
                    ("$bin", "pyc"),
                    ("$stdin", "in"),
                    ("$stdout", "out"),
                ] // public
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
                run: ["cat $stdin | python ./golem.py $bin"] // public
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),

                mapped_dir: Bind {
                    from: PathBuf::from(r"../golem/src"),
                    to: PathBuf::from(r"../golem/src"),
                    readonly: false,
                },
                binds: Some(vec![]),
                special_judge_script: None,
            },
            &JudgeTomlTestConfig {
                // TODO: Refine interface
                image: Image::Image { tag: "".into() },
                build: None,
                run: vec!["python ./golemc.py $src -o $bin".into()],
            },
            TestSuiteOptions {
                tests: ["succ"].iter().map(|s| s.to_string()).collect(), // private
                time_limit: None,                                        // private
                mem_limit: None,                                         // private
                build_image: true,                                       // private
                remove_image: true,                                      // private
            },
        )
        .await?;

        let instance = bollard::Docker::connect_with_local_defaults().unwrap();
        ts.run(
            instance,
            std::env::current_dir().unwrap(),
            None,
            None,
            None,
            Default::default(),
        )
        .await?;
        Ok(())
    })
}
