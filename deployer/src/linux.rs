use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Step 1: Run the Python script under 'strace' to record every file access.
pub fn run_spy(python_script: &Path, test_image: &Path, venv_path: &Path) -> Result<PathBuf> {
    let python_script = python_script.canonicalize().context("Python script not found")?;
    let test_image = test_image.canonicalize().context("Test image not found (did you add one to assets/?)")?;
    let venv_path = venv_path.canonicalize().context("Venv not found")?;

    println!("🕵️  [Linux] Starting Spy...");
    println!("    Script: {:?}", python_script);
    println!("    Image:  {:?}", test_image);

    let log_file = Path::new("strace.log");
    
    // We use the python interpreter INSIDE the venv
    let python_bin = venv_path.join("bin/python3");
    
    // Command: strace -f -e trace=open,openat -o strace.log python3 ppocr.py image.png
    let status = Command::new("strace")
        .arg("-f") // Follow child processes (crucial for multiprocessing)
        .arg("-e")
        .arg("trace=open,openat") // Only log file opening
        .arg("-o")
        .arg(log_file)
        .arg(&python_bin)
        .arg(&python_script)
        .arg(&test_image)
        .status()
        .context("Failed to run strace. Is it installed? (sudo apt install strace)")?;

    if !status.success() {
        anyhow::bail!("Spy execution failed! The script crashed during execution. Check strace.log for Python errors.");
    }

    println!("✅ Spy finished. Log saved to {:?}", log_file.canonicalize()?);
    Ok(log_file.to_path_buf())
}

/// Step 2: Parse the strace log and copy ONLY the used files from the venv.
pub fn harvest_deps(log_file: &Path, venv_path: &Path, output_dir: &Path) -> Result<()> {
    println!("🚜 [Linux] Harvesting dependencies...");

    // Regex to find paths in double quotes: openat(..., "/path/to/file", ...)
    let re = Regex::new(r#""(/[^"]+)""#).unwrap();
    
    let file = fs::File::open(log_file).context("Could not open strace.log")?;
    let reader = BufReader::new(file);

    let mut files_to_copy = HashSet::new();
    let venv_str = venv_path.canonicalize()?.to_string_lossy().to_string();

    // 1. Parse Log
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = re.captures(&line) {
            let path_str = &caps[1];
            
            // FILTER: 
            // 1. Must be inside our Venv
            // 2. Ignore __pycache__ (byte code is regenerated anyway)
            // 3. Must be a file that actually exists
            if path_str.starts_with(&venv_str) && !path_str.contains("__pycache__") {
                let p = PathBuf::from(path_str);
                if p.exists() && p.is_file() {
                    files_to_copy.insert(p);
                }
            }
        }
    }

    println!("    Found {} unique files to harvest.", files_to_copy.len());

    // 2. Clean Output Dir
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    // 3. Copy Files
    let mut copied_count = 0;
    for src in files_to_copy {
        // We need to preserve the folder structure relative to 'site-packages'.
        // Example: /.../venv/lib/python3.10/site-packages/paddle/dataset/image.py
        // Target:  dist/lib/paddle/dataset/image.py
        
        let src_str = src.to_string_lossy();
        
        // Find where "site-packages" starts
        if let Some(idx) = src_str.find("/site-packages/") {
            // +15 is the length of "/site-packages/"
            let rel_part = &src_str[idx + 15..]; 
            let dest = output_dir.join(rel_part);

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Use copy, not symlink, so it's portable
            fs::copy(&src, &dest)?;
            copied_count += 1;
        } 
        // OPTIONAL: Handle libs outside site-packages if needed (rare for venv)
    }

    println!("📦 Harvest complete! {} files copied to {:?}", copied_count, output_dir);
    Ok(())
}
