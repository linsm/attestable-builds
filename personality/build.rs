use std::{fs, path::Path};

const TRILLIAN_BASE_URL: &str =
    "https://raw.githubusercontent.com/google/trillian/5979df6e8e907186d0f503ce029ef72a334e3524/";
const GOOGLE_RPC_BASE_URL: &str = "https://raw.githubusercontent.com/googleapis/googleapis/a26064a9cc78d4518b8a9fd2ea78891edad4d87d/google/rpc/";

const TRILLIAN_FILES: &[&str] = &[
    "trillian_admin_api.proto",
    "trillian_log_api.proto",
    "trillian.proto",
];

const GOOGLE_RPC_FILES: &[&str] = &["status.proto"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    download_proto_files();
    tonic_build::configure()
        .build_server(false)
        .out_dir("src/trillian_rust")
        .include_file("mod.rs")
        .compile_protos(
            &[
                "trillian/trillian.proto",
                "trillian/trillian_log_api.proto",
                "trillian/trillian_admin_api.proto",
                "google/rpc/status.proto",
            ],
            &[
                "trillian",
                "trillian_log_api",
                "trillian_admin_api",
                "google",
            ],
        )?;
    Ok(())
}

fn download_proto_files() {
    for trillian_file in TRILLIAN_FILES {
        let path = format!("trillian/{trillian_file}");
        if !Path::new(&path).exists() {
            let url = format!("{TRILLIAN_BASE_URL}{trillian_file}");
            let content = reqwest::blocking::get(url).expect("error downloading proto files");
            fs::write(path, content.text().unwrap()).expect("error while writing proto file");
        }
    }
    for google_rpc_file in GOOGLE_RPC_FILES {
        let path = format!("trillian/google/rpc/{google_rpc_file}");
        if !Path::new(&path).exists() {
            fs::create_dir_all("trillian/google/rpc/").expect("failed to create sub directories");
            let url = format!("{GOOGLE_RPC_BASE_URL}{google_rpc_file}");
            let content = reqwest::blocking::get(url).expect("error downloading proto files");
            fs::write(path, content.text().unwrap()).expect("error while writing proto file");
        }
    }
}
