use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
struct Stats {
    bytes: u64,
    lines: u64,
}

impl Stats {
    fn from_path(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut bytes = 0u64;
        let mut lines = 0u64;

        loop {
            let mut buf = String::new();
            let len = reader.read_line(&mut buf)?;
            if len == 0 {
                break;
            }
            lines += 1;
            bytes += len as u64;
        }
        Ok(Self { bytes, lines })
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <md_dir> <rs_dir>", args[0]);
        std::process::exit(1);
    }

    let md_root = Path::new(&args[1]);
    let rs_root = Path::new(&args[2]);

    if !md_root.is_dir() {
        eprintln!("Error: '{}' is not a directory", md_root.display());
        std::process::exit(1);
    }
    if !rs_root.is_dir() {
        eprintln!("Error: '{}' is not a directory", rs_root.display());
        std::process::exit(1);
    }

    let mut total_md = Stats::default();
    let mut total_matching_rs = Stats::default();
    let mut unmatched_rs = Stats::default();
    let mut unmatched_count = 0usize;

    visit_md_dir(md_root, md_root, rs_root, &mut total_md, &mut total_matching_rs);

    let mut unmatched_rs_files = Vec::new();
    visit_rs_dir(rs_root, rs_root, md_root, &mut unmatched_rs, &mut unmatched_count, &mut unmatched_rs_files);

    println!("=== Summary ===");
    println!("Total .md files: bytes={}, lines={}", total_md.bytes, total_md.lines);
    println!("Total matching .rs files: bytes={}, lines={}", total_matching_rs.bytes, total_matching_rs.lines);
    println!("Unmatched .rs files: {} files, bytes={}, lines={}", unmatched_count, unmatched_rs.bytes, unmatched_rs.lines);

    if !unmatched_rs_files.is_empty() {
        println!("\n=== Unmatched .rs files ===");
        for (path, stats) in unmatched_rs_files {
            println!("{}: bytes={}, lines={}", path.display(), stats.bytes, stats.lines);
        }
    }
}

fn visit_md_dir(
    dir: &Path,
    md_root: &Path,
    rs_root: &Path,
    total_md: &mut Stats,
    total_matching_rs: &mut Stats,
) {
    let walker = match fs::read_dir(dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error reading dir {}: {}", dir.display(), e);
            return;
        }
    };

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error enumerating {}: {}", dir.display(), e);
                continue;
            }
        };
        let path = entry.path();
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error accessing {}: {}", path.display(), e);
                continue;
            }
        };

        if meta.is_dir() {
            visit_md_dir(&path, md_root, rs_root, total_md, total_matching_rs);
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let rs_path = build_rs_path(&path, md_root, rs_root);
            let md_stats = match Stats::from_path(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                    continue;
                }
            };
            *total_md = Stats {
                bytes: total_md.bytes + md_stats.bytes,
                lines: total_md.lines + md_stats.lines,
            };

            if !rs_path.exists() {
                eprintln!("Warning: matching .rs file not found: {}", rs_path.display());
                continue;
            }
            let rs_stats = match Stats::from_path(&rs_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", rs_path.display(), e);
                    continue;
                }
            };
            *total_matching_rs = Stats {
                bytes: total_matching_rs.bytes + rs_stats.bytes,
                lines: total_matching_rs.lines + rs_stats.lines,
            };
            println!("Pair: {} .md -> {} .rs", path.display(), rs_path.display());
            println!("  md:  bytes={}, lines={}", md_stats.bytes, md_stats.lines);
            println!("  rs:  bytes={}, lines={}", rs_stats.bytes, rs_stats.lines);
        }
    }
}

fn visit_rs_dir(
    dir: &Path,
    rs_root: &Path,
    md_root: &Path,
    unmatched_rs: &mut Stats,
    unmatched_count: &mut usize,
    unmatched_rs_files: &mut Vec<(PathBuf, Stats)>,
) {
    let walker = match fs::read_dir(dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Error reading dir {}: {}", dir.display(), e);
            return;
        }
    };

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error enumerating {}: {}", dir.display(), e);
                continue;
            }
        };
        let path = entry.path();
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error accessing {}: {}", path.display(), e);
                continue;
            }
        };

        if meta.is_dir() {
            visit_rs_dir(&path, rs_root, md_root, unmatched_rs, unmatched_count, unmatched_rs_files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let md_path = build_md_path(&path, rs_root, md_root);
            if !md_path.exists() {
                let stats = match Stats::from_path(&path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error reading {}: {}", path.display(), e);
                        continue;
                    }
                };
                unmatched_rs.bytes += stats.bytes;
                unmatched_rs.lines += stats.lines;
                *unmatched_count += 1;
                unmatched_rs_files.push((path, stats));
            }
        }
    }
}

fn build_rs_path(md_path: &Path, md_root: &Path, rs_root: &Path) -> PathBuf {
    let rel = md_path.strip_prefix(md_root).unwrap();
    let mut new_stem = rel.to_path_buf();
    new_stem.set_extension("rs");
    rs_root.join(new_stem)
}

fn build_md_path(rs_path: &Path, rs_root: &Path, md_root: &Path) -> PathBuf {
    let rel = rs_path.strip_prefix(rs_root).unwrap();
    let mut new_stem = rel.to_path_buf();
    new_stem.set_extension("md");
    md_root.join(new_stem)
}