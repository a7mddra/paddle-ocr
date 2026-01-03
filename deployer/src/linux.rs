use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_spy(python_script: &Path, test_image: &Path, venv_path: &Path) -> Result<PathBuf> {
    println!("🕵️  [Linux] Starting Spy on {:?}", python_script);

    let log_file = Path::new("strace.log");
    
    // 1. Construct the Python command using the venv's python
    let python_bin = venv_path.join("bin/python3");
    
    // 2. Run strace
    // strace -f -e trace=open,openat -o strace.log python3 script.py image.png
    let status = Command::new("strace")
        .arg("-f")
        .arg("-e")
        .arg("trace=open,openat")
        .arg("-o")
        .arg(log_file)
        .arg(&python_bin)
        .arg(python_script)
        .arg(test_image)
        .status()
        .context("Failed to run strace. Is it installed?")?;

    if !status.success() {
        anyhow::bail!("Spy execution failed. Check if the script runs normally first.");
    }

    println!("✅ Spy finished. Log saved to {:?}", log_file);
    Ok(log_file.to_path_buf())
}

pub fn harvest_deps(log_file: &Path, venv_path: &Path, output_dir: &Path) -> Result<()> {
    println!("🚜 [Linux] Harvesting dependencies from {:?}", log_file);

    // 1. Prepare Regex to extract paths from strace logs
    // Example line: openat(AT_FDCWD, "/path/to/venv/lib/...", O_RDONLY|O_CLOEXEC) = 3
    let re = Regex::new(r#""(/[^"]+)""#).unwrap();
    
    let file = fs::File::open(log_file).context("Could not open trace log")?;
    let reader = BufReader::new(file);

    let mut files_to_copy = HashSet::new();
    let venv_str = venv_path.canonicalize()?.to_string_lossy().to_string();

    // 2. Parse the log
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = re.captures(&line) {
            let path_str = &caps[1];
            
            // Filter: Must be inside the VENV and must exist
            if path_str.starts_with(&venv_str) && !path_str.contains("pycache") {
                let p = PathBuf::from(path_str);
                if p.exists() && p.is_file() {
                    files_to_copy.insert(p);
                }
            }
        }
    }

    println!("Found {} unique files to harvest.", files_to_copy.len());

    // 3. Copy files
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    for src in files_to_copy {
        // Calculate relative path inside venv
        // e.g. /venv/lib/python3.10/site-packages/numpy/... -> numpy/...
        // We need to be careful about stripping the prefix.
        
        // Strategy: find "site-packages" and take everything after it
        let src_str = src.to_string_lossy();
        if let Some(idx) = src_str.find("site-packages/") {
            let rel_part = &src_str[idx + 14..]; // 14 is len("site-packages/")
            let dest = output_dir.join(rel_part);

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dest)?;
        } else {
            // Handle non-site-packages (like configs or base libs) if necessary
            // For now, we focus on site-packages
        }
    }

    println!("📦 Harvest complete in {:?}", output_dir);
    Ok(())
}
