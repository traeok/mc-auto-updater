extern crate colored;
extern crate directories;

use colored::*;
use serde::{Deserialize, Serialize};

use std::{
    fs::File,
    io::{Error, Write},
    path::Path,
    time,
};

use directories::BaseDirs;

#[derive(Serialize, Deserialize)]
struct ModpackData {
    modpack: String,
    modpack_dir: String,
    version: String,
    changelog: String,
    blacklist: Vec<String>,
    mods_url: String,
}

fn check_for_updates(desired_path: &Path) -> Result<(), Error> {
    let metadata_url = match option_env!("MC_MODPACK_METADATA_URL") {
        Some(url) => url,
        None => "https://trae.is/mc_metadata.json",
    };

    let response = match reqwest::blocking::get(metadata_url) {
        Ok(resp) => resp.text().unwrap(),
        Err(why) => {
            println!("Failed to fetch modpack metadata, error: {why}");
            std::thread::sleep(time::Duration::from_secs(3));
            std::process::exit(1);
        }
    };

    let metadata: ModpackData = serde_json::from_str(&response).unwrap();

    println!("{}", "mc-auto-updater by @gh/traeok\nv1.2.0".bold());
    println!("{} {}\n", "Current Modpack:".bold(), metadata.modpack);

    let mod_path = desired_path
        .join("instances")
        .join(metadata.modpack_dir)
        .join("mods");

    let version_txt_path = mod_path.join("version.txt");
    let version_txt = std::fs::read_to_string(&version_txt_path).unwrap_or("N/A".to_string());
    if version_txt == metadata.version {
        println!("{}", "Up to date. Closing in 3 seconds...".green());
        std::thread::sleep(time::Duration::from_secs(3));
        std::process::exit(0);
    }

    println!(
        "Your version is out of date.\nLatest version available: {}\n",
        metadata.version
    );

    match File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&version_txt_path)
    {
        Ok(mut file) => {
            file.write_all(metadata.version.as_bytes())?;
            file.flush()?;
        }
        Err(error) => {
            println!("Failed to open a handle to version.txt: {error:?}");
            std::thread::sleep(time::Duration::from_secs(3));
            return Err(error);
        }
    }

    println!("Changelog:\n{}\n", metadata.changelog.yellow());

    let response = match reqwest::blocking::get(metadata.mods_url) {
        Ok(resp) => resp,
        Err(error) => {
            println!("Cannot fetch mods.zip, error: {error}");
            std::thread::sleep(time::Duration::from_secs(3));
            std::process::exit(1);
        }
    };

    let content = response.bytes().unwrap();
    let mod_zip_path = mod_path.join("mods.zip");
    let mut mod_zip = match File::options()
        .create(true)
        .read(true)
        .write(true)
        .open(&mod_zip_path)
    {
        Err(why) => {
            println!("Couldn't create mods.zip, error: {why:?}");
            std::thread::sleep(time::Duration::from_secs(3));
            return Err(why);
        }
        Ok(file) => file,
    };
    mod_zip.write_all(&content)?;

    let mut archive = zip::ZipArchive::new(mod_zip).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => mod_path.join(path.to_owned()),
            None => continue,
        };

        if outpath.exists() {
            continue;
        }

        if (*file.name()).ends_with('/') {
            std::fs::create_dir_all(&outpath).unwrap();
        } else {
            println!("{} {}", "[+]".green(), outpath.display());
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = std::fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }
    }
    std::fs::remove_file(mod_zip_path)?;

    for file in metadata.blacklist {
        if !file.is_empty() {
            let file_path = mod_path.join(file);
            if file_path.exists() {
                println!("{} {}", "[-]".red(), file_path.display());
                std::fs::remove_file(file_path)?;
            }
        }
    }

    println!(
        "{} {} {}",
        "Done! Updated to version".green(),
        metadata.version,
        "\nExiting in 5 seconds...".green(),
    );
    std::thread::sleep(time::Duration::from_secs(5));
    return Ok(());
}
fn main() -> Result<(), Error> {
    #[cfg(target_os = "windows")]
    control::set_virtual_terminal(true).unwrap();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "here" {
        return check_for_updates(&std::env::current_dir().unwrap());
    }

    if let Some(base_dirs) = BaseDirs::new() {
        return check_for_updates(&base_dirs.data_dir().join("ATLauncher"));
    }

    let last_error = std::io::Error::last_os_error();
    println!("An error occurred while running mc-auto-updater: {last_error:?}");
    return Err(last_error);
}
