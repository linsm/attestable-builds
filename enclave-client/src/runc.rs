use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConfigJson {
    #[serde(rename = "ociVersion")]
    oci_version: String,
    #[serde(rename = "process")]
    process: Process,
    #[serde(rename = "root")]
    root: Root,
    #[serde(rename = "hostname")]
    hostname: String,
    #[serde(rename = "mounts")]
    mounts: Vec<Mount>,
    #[serde(rename = "linux")]
    linux: Linux,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Process {
    #[serde(rename = "terminal")]
    terminal: bool,
    #[serde(rename = "user")]
    user: User,
    #[serde(rename = "args")]
    args: Vec<String>,
    #[serde(rename = "env")]
    env: Vec<String>,
    #[serde(rename = "cwd")]
    cwd: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct User {
    #[serde(rename = "uid")]
    pub(crate) uid: u32,
    #[serde(rename = "gid")]
    pub(crate) gid: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Root {
    #[serde(rename = "path")]
    path: String,
    #[serde(rename = "readonly")]
    readonly: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Mount {
    #[serde(rename = "destination")]
    pub destination: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "source")]
    pub source: String,
    #[serde(rename = "options")]
    pub options: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Linux {
    #[serde(rename = "resources")]
    resources: Resources,
    #[serde(rename = "namespaces")]
    namespaces: Vec<Namespace>,
    #[serde(rename = "maskedPaths")]
    masked_paths: Vec<String>,
    #[serde(rename = "readonlyPaths")]
    readonly_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Resources {
    #[serde(rename = "devices")]
    devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Device {
    #[serde(rename = "allow")]
    allow: bool,
    #[serde(rename = "access")]
    access: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Namespace {
    #[serde(rename = "type")]
    type_: String,
    #[serde(rename = "path")]
    path: Option<String>,
}

pub fn patch_config_json(
    config_json: ConfigJson,
    args: Option<String>,
    additional_envs: Option<Vec<String>>,
    user: Option<User>,
    cwd: Option<String>,
    additional_mounts: Option<Vec<Mount>>,
) -> ConfigJson {
    let mut config_json = config_json;

    if let Some(args) = args {
        config_json.process.args = args.split_whitespace().map(|s| s.to_string()).collect();
    }

    if let Some(additional_envs) = additional_envs {
        config_json.process.env.extend(additional_envs);
    }

    if let Some(user) = user {
        config_json.process.user = user;
    }

    if let Some(cwd) = cwd {
        config_json.process.cwd = cwd;
    }

    if let Some(additional_mounts) = additional_mounts {
        config_json.mounts.extend(additional_mounts);
    }

    config_json
}

#[cfg(test)]
mod tests {
    use crate::runc::ConfigJson;
    use std::path::PathBuf;

    #[test]
    fn test_deserialize_config_json() {
        let path = get_sample_config_json_path();

        let config_json_string = std::fs::read_to_string(path).unwrap();
        let config_json: ConfigJson = serde_json::from_str(&config_json_string).unwrap();

        assert_eq!(config_json.root.path, "rootfs");
    }

    #[test]
    fn test_serde_config_json_then_equal() {
        let path = get_sample_config_json_path();

        let config_json_string = std::fs::read_to_string(path).unwrap();
        let expected: ConfigJson = serde_json::from_str(&config_json_string).unwrap();

        let serialized = serde_json::to_string(&expected).unwrap();
        let actual: ConfigJson = serde_json::from_str(&serialized).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_patch_noop() {
        let path = get_sample_config_json_path();

        let config_json_string = std::fs::read_to_string(path).unwrap();
        let config_json: ConfigJson = serde_json::from_str(&config_json_string).unwrap();

        let expected = config_json.clone();
        let actual = super::patch_config_json(config_json, None, None, None, None, None);

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_patch_all_parameters() {
        let path = get_sample_config_json_path();

        let config_json_string = std::fs::read_to_string(path).unwrap();
        let config_json: ConfigJson = serde_json::from_str(&config_json_string).unwrap();

        let args = Some("echo hello".to_string());
        let env = Some(vec!["FOO=bar".to_string()]);
        let user = Some(super::User {
            uid: 1000,
            gid: 1000,
        });
        let cwd = Some("/tmp".to_string());
        let additional_mounts = Some(vec![super::Mount {
            destination: "/tmp".to_string(),
            type_: "bind".to_string(),
            source: "/tmp".to_string(),
            options: None,
        }]);

        let mut expected_mounts = config_json.mounts.clone();
        expected_mounts.extend(additional_mounts.clone().unwrap());

        let mut expected_envs = config_json.process.env.clone();
        expected_envs.extend(env.clone().unwrap());

        let expected = ConfigJson {
            process: super::Process {
                args: vec!["echo".to_string(), "hello".to_string()],
                env: expected_envs,
                user: super::User {
                    uid: 1000,
                    gid: 1000,
                },
                cwd: "/tmp".to_string(),
                ..config_json.process
            },
            mounts: expected_mounts,
            ..config_json.clone()
        };

        let actual = super::patch_config_json(config_json, args, env, user, cwd, additional_mounts);
        assert_eq!(expected, actual);
    }

    fn get_sample_config_json_path() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join(PathBuf::from("../sandbox-container/config.base.json"))
    }
}
