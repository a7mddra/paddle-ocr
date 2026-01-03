mod linux;
mod mac;
mod win;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "deployer")]
#[command(about = "Builds the PaddleOCR standalone engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Step 1: Run the Python script under surveillance to detect used files
    Spy {
        #[arg(long)]
        venv: PathBuf,
        #[arg(long)]
        script: PathBuf,
        #[arg(long)]
        image: PathBuf,
    },
    /// Step 2: Extract detected files to a clean libs folder
    Harvest {
        #[arg(long)]
        venv: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Step 3: Compile the bootloader using PyInstaller
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
        }
        Commands::Harvest { venv, output } => {
            let log_file = PathBuf::from("strace.log");
            #[cfg(target_os = "linux")]
            linux::harvest_deps(&log_file, venv, output)?;
        }
        Commands::Build { venv, bootloader } => {
            println!("🔨 Building bootloader with PyInstaller...");
            // We call PyInstaller via the venv's pip
            let python_bin = venv.join("bin/python3"); // adjust for windows
            let pyinstaller = venv.join("bin/pyinstaller"); 
            
            let status = std::process::Command::new(pyinstaller)
                .arg("--noconfirm")
                .arg("--onefile")
                .arg("--clean")
                .arg("--name").arg("ocr-engine")
                // Exclude the heavy stuff we just harvested
                .arg("--exclude-module").arg("paddle")
                .arg("--exclude-module").arg("paddleocr")
                .arg("--exclude-module").arg("numpy")
                .arg("--exclude-module").arg("PIL")
                .arg("--exclude-module").arg("cv2")
                .arg(bootloader)
                .status()?;

            if status.success() {
                println!("✅ Build Success! Binary is in dist/");
            } else {
                anyhow::bail!("PyInstaller failed");
            }
        }
    }

    Ok(())
}
