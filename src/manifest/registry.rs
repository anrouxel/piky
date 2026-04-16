use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Registry {
    registry: RegistryInfo,
    plugins: HashMap<String, RegistryEntry>
}

#[derive(Debug, Deserialize)]
struct RegistryInfo {
    name: String,
    description: String
}

#[derive(Debug, Deserialize)]
enum RegistryEntry {
    Url(String)
}