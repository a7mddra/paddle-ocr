mod linux;
// mod mac; // Placeholder for future expansion
// mod win; // Placeholder for future expansion

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "deployer")]
#[command(about = "PaddleOCR Standalone Builder", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Step 1: Run the script with a real image to detect dependencies
    Spy {
        #[arg(long)]
        venv: PathBuf,
        #[arg(long)]
        script: PathBuf,
        #[arg(long)]
        image: PathBuf,
    },
    /// Step 2: Copy the detected dependencies to a folder
    Harvest {
        #[arg(long)]
        venv: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Step 3: Build the lightweight bootloader executable
    Build {
        #[arg(long)]
        venv: PathBuf,
        #[arg(long)]
        bootloader: PathBuf,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Spy { venv, script, image } => {
            #[cfg(target_os = "linux")]
            linux::run_spy(script, image, venv)?;
            
            #[cfg(not(target_os = "linux"))]
            println!("Spy is currently Linux-only via strace.");
        }
        Commands::Harvest { venv, output } => {
            let log_file = PathBuf::from("strace.log");
            
            #[cfg(target_os = "linux")]
            linux::harvest_deps(&log_file, venv, output)?;
        }
        Commands::Build { venv, bootloader } => {
            println!("🔨 Building bootloader with PyInstaller...");
            
            let _python_bin = venv.join("bin/python3");
            let pyinstaller = venv.join("bin/pyinstaller");
            
            // We EXCLUDE the heavy libraries because we just harvested them manually!
            // This forces PyInstaller to create a tiny 15MB binary (Just Python + StdLib).
            let status = std::process::Command::new(pyinstaller)
                .arg("--noconfirm")
                .arg("--onefile")
                .arg("--clean")
                .arg("--name").arg("ocr-engine")
                .arg("--exclude-module").arg("paddle")
                .arg("--exclude-module").arg("paddleocr")
                .arg("--exclude-module").arg("numpy")
                .arg("--exclude-module").arg("PIL")
                .arg("--exclude-module").arg("cv2")
                .arg("--exclude-module").arg("matplotlib")
                .arg(bootloader)
                .status()?;

            if status.success() {
                println!("✅ Build Success!");
                println!("   Binary: dist/ocr-engine");
                println!("   Libs:   (Your harvest folder)");
                println!("   Script: (Copy ppocr.py manually)");
            } else {
                anyhow::bail!("PyInstaller execution failed.");
            }
        }
    }

    Ok(())
}
