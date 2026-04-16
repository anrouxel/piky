use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Plugin {
    package: PackageInfo
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
    description: String,
}