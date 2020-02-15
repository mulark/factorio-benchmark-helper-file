use std::collections::HashMap;
use core::str::FromStr;
use std::fs::read;
use core::ops::Not;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TopLevel {
    pub benchmark_sets: BTreeMap<String, BenchmarkSet>,
    pub meta_sets: BTreeMap<String, BTreeSet<String>>,
}

impl TopLevel {
    pub fn print_summary(&self, kinds: ProcedureKind) {
        if kinds == ProcedureKind::Benchmark || kinds == ProcedureKind::Both {
            println!("    Benchmark Sets:");
            for set in self.benchmark_sets.keys() {
                println!("\t{:?}", set);
            }
        }
        if kinds == ProcedureKind::Meta || kinds == ProcedureKind::Both {
            println!("    Meta Sets:");
            for set in self.meta_sets.keys() {
                println!("\t{:?}", set);
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BenchmarkSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_subdirectory: Option<PathBuf>,
    pub mods: BTreeSet<Mod>,
    pub maps: BTreeSet<Map>,
    pub ticks: u32,
    pub runs: u32,
}

impl Default for BenchmarkSet {
    fn default() -> BenchmarkSet {
        BenchmarkSet {
            save_subdirectory: None,
            mods: BTreeSet::new(),
            maps: BTreeSet::new(),
            ticks: 0,
            runs: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, Eq, PartialOrd)]
pub struct Map {
    pub name: String,
    #[serde(skip)]
    pub path: PathBuf,
    pub sha256: String,
    pub download_link: String,
}

impl Map {
    pub fn new(path: &PathBuf, sha256: &str, download_link: &str) -> Map {
        Map {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_path_buf(),
            sha256: sha256.to_string(),
            download_link: download_link.to_string(),
        }
    }
}

impl PartialEq for Map {
    fn eq(&self, cmp: &Self) -> bool {
        if self.sha256 == cmp.sha256 {
            return true;
        }
        false
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Ord, Eq)]
pub struct Mod {
    pub name: String,
    #[serde(skip)]
    pub file_name: String,
    pub version: String,
    pub sha1: String,
}

impl Mod {
    pub fn new(name: &str, file_name: &str, version: &str, hash: &str) -> Mod {
        Mod {
            name: name.to_string(),
            file_name: file_name.to_string(),
            version: version.to_string(),
            sha1: hash.to_string(),
        }
    }
}

impl PartialEq for Mod {
    fn eq(&self, cmp: &Self) -> bool {
        if self.sha1 == cmp.sha1 && !self.sha1.is_empty() {
            return true;
        }
        false
    }
}

#[derive(Debug, PartialEq)]
pub enum ProcedureError {
    ProcedureAlreadyExists,
    FileNotFound,
    MalformedJSON,
    UnknownReadError,
}

#[derive(Debug, PartialEq)]
pub enum ProcedureKind {
    Benchmark,
    Meta,
    Both,
}


/// When writing to a file, if there is already a procedure with the same name, should this procedure be overwritten?
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ProcedureOverwrite {
    True,
    False,
}

impl From<bool> for ProcedureOverwrite {
    fn from(b: bool) -> ProcedureOverwrite {
        if b {
            ProcedureOverwrite::True
        } else {
            ProcedureOverwrite::False
        }
    }
}

impl Not for ProcedureOverwrite {
    type Output = ProcedureOverwrite;
    fn not(self) -> Self::Output {
        match self {
            ProcedureOverwrite::True => ProcedureOverwrite::False,
            ProcedureOverwrite::False => ProcedureOverwrite::True,
        }
    }
}


pub fn load_top_level_from_file(file: &Path) -> Result<TopLevel,ProcedureError> {
    if file.exists() {
        if let Ok(bytes) = &read(file) {
            if let Ok(json) = serde_json::from_slice(bytes) {
                Ok(json)
            } else {
                Err(ProcedureError::MalformedJSON)
            }
        } else {
            Err(ProcedureError::UnknownReadError)
        }
    } else {
        Err(ProcedureError::FileNotFound)
    }
}

impl FromStr for ProcedureKind {
    type Err = String;
    fn from_str(s: &str) -> Result<ProcedureKind, Self::Err> {
        match s.to_lowercase().as_str() {
            "benchmark" => Ok(ProcedureKind::Benchmark),
            "meta" => Ok(ProcedureKind::Meta),
            "both" => Ok(ProcedureKind::Both),
            _ => Err(String::from("Error: UnknownProcedureType")),
        }
    }
}

/// Reads a benchmark set from a file, returning None if the file doesn't exist or doesn't contain the supplied benchmark set
pub fn read_benchmark_set_from_file(
    name: &str,
    file: &Path,
) -> Option<BenchmarkSet> {
    if let Ok(m) = load_top_level_from_file(file) {
        if m.benchmark_sets.contains_key(name) {
            return Some(m.benchmark_sets[name].clone());
        }
    }
    None
}

/// Writes a benchmark set to a file, if supplied file does not exist it will created
pub fn write_benchmark_set_to_file(
    set_name: &str,
    set: BenchmarkSet,
    overwrite: ProcedureOverwrite,
    file: &Path,
) -> Result<(),ProcedureError> {
    let mut top_level;
    match load_top_level_from_file(&file) {
        Ok(m) => {
            top_level = m;
        }
        _ => {
            top_level = TopLevel::default();
        }
    }
    if top_level.benchmark_sets.contains_key(set_name) && overwrite == false.into() {
        return Err(ProcedureError::ProcedureAlreadyExists);
    } else {
        top_level.benchmark_sets.insert(set_name.to_string(), set);
        let j = serde_json::to_string_pretty(&top_level).unwrap();
        std::fs::write(file, j).unwrap();
    }
    Ok(())
}

pub fn read_meta_from_file(name: &str, file: &Path) -> Option<BTreeSet<String>> {
    match load_top_level_from_file(&file) {
        Ok(m) => {
            if m.meta_sets.contains_key(name) {
                return Some(m.meta_sets[name].clone());
            }
        }
        _ => return None,
    }
    None
}

pub fn write_meta_to_file(
    name: &str,
    members: BTreeSet<String>,
    force: ProcedureOverwrite,
    file: &Path,
) -> Result<(),ProcedureError> {
    let mut top_level;
    match load_top_level_from_file(&file) {
        Ok(m) => top_level = m,
        _ => top_level = TopLevel::default(),
    }

    if top_level.meta_sets.contains_key(name) && force == false.into() {
        return Err(ProcedureError::ProcedureAlreadyExists);
    } else {
        top_level.meta_sets.insert(name.to_string(), members);
        let j = serde_json::to_string_pretty(&top_level).unwrap();
        std::fs::write(file, j).unwrap();
    }
    Ok(())
}

// Returns a hashmap of all benchmark sets contained within this meta set, as well as the meta sets
// found recursively within meta sets contained within this meta set.
pub fn get_sets_from_meta(
    meta_set_key: String,
    file: &Path,
) -> HashMap<String, BenchmarkSet> {
    let mut current_sets = HashMap::new();
    let mut seen_keys = Vec::new();
    let top_level = load_top_level_from_file(&file).unwrap();
    walk_meta_recursive_for_benchmarks(meta_set_key, &top_level, &mut seen_keys, &mut current_sets);
    current_sets
}

fn walk_meta_recursive_for_benchmarks(
    key: String,
    top_level: &TopLevel,
    seen_keys: &mut Vec<String>,
    current_benchmark_sets: &mut HashMap<String, BenchmarkSet>,
) {
    if !seen_keys.contains(&key) {
        if top_level.meta_sets.contains_key(&key) {
            seen_keys.push(key.clone());
            for k in &top_level.meta_sets[&key] {
                walk_meta_recursive_for_benchmarks(
                    k.to_string(),
                    &top_level,
                    seen_keys,
                    current_benchmark_sets,
                );
            }
        }
        if top_level.benchmark_sets.contains_key(&key) {
            current_benchmark_sets.insert(key.clone(), top_level.benchmark_sets[&key].to_owned());
        }
    }
}

pub fn get_metas_from_meta(
    meta_set_key: String,
    file: &Path,
) -> Result<Vec<String>,ProcedureError> {
    let mut seen_keys = Vec::new();
    let mut current_meta_sets = Vec::new();
    let top_level = load_top_level_from_file(&file)?;
    walk_meta_recursive_for_metas(
        meta_set_key,
        &top_level,
        &mut seen_keys,
        &mut current_meta_sets,
    );
    Ok(current_meta_sets)
}

fn walk_meta_recursive_for_metas(
    key: String,
    top_level: &TopLevel,
    seen_keys: &mut Vec<String>,
    current_meta_sets: &mut Vec<String>,
) {
    if !seen_keys.contains(&key) && top_level.meta_sets.contains_key(&key) {
        seen_keys.push(key.clone());
        for k in &top_level.meta_sets[&key] {
            walk_meta_recursive_for_metas(k.to_string(), &top_level, seen_keys, current_meta_sets);
        }
        current_meta_sets.push(key);
    }
}
