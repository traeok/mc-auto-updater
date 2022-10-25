extern crate colored;
extern crate directories;

use colored::*;

use std::{
    fs::File,
    io::{Error, Write},
    path::Path,
    time,
};

use directories::BaseDirs;

fn check_for_updates(desired_path: &Path) -> Result<(), Error> {
    println!("{}", "mc-auto-updater by @gh/traeok\nv1.1.1".bold());
    println!("{}", "Current Modpack: All the Mods 6 (1.16.5)\n".bold());

    let version_url = match option_env!("MC_VERSION_URL") {
        Some(url) => url,
        None => "https://trae.is/version.txt",
    };

    let response = reqwest::blocking::get(version_url).unwrap();
    let version = response.text().unwrap();

    let mod_path = desired_path
        .join("instances")
        .join("AlltheMods6ATM61165")
        .join("mods");

    let version_txt_path = mod_path.join("version.txt");
    let version_txt = std::fs::read_to_string(&version_txt_path).unwrap_or("N/A".to_string());
    if version_txt == version {
        println!("{}", "Up to date. Closing in 3 seconds...".green());
        std::thread::sleep(time::Duration::from_secs(3));
        std::process::exit(0);
    }

    println!(
        "Your version is out of date.\nLatest version available: {}\n",
        version
    );

    match File::options()
        .create(true)
        .write(true)
        .open(&version_txt_path)
    {
        Ok(mut file) => {
            file.write_all(version.as_bytes())?;
            file.flush()?;
        }
        Err(error) => {
            println!("Failed to open a handle to version.txt: {error:?}");
            std::thread::sleep(time::Duration::from_secs(3));
            return Err(error);
        }
    }

    let mc_changelog_url = match option_env!("MC_CHANGELOG_URL") {
        Some(url) => url,
        None => "https://trae.is/mc_changelog.txt",
    };
    let response = reqwest::blocking::get(mc_changelog_url).unwrap();
    let changelog = response.text().unwrap();
    println!("Changelog:\n{}\n", changelog.yellow());

    let mc_mods_url = match option_env!("MC_MODS_URL") {
        Some(url) => url,
        None => "https://trae.is/mods.zip",
    };
    let response = reqwest::blocking::get(mc_mods_url).unwrap();
    let content = response.bytes().unwrap();
    let mod_zip_path = mod_path.join("mods.zip");
    let mut mod_zip = match File::options()
        .create(true)
        .read(true)
        .write(true)
        .open(&mod_zip_path)
    {
        Err(why) => {
            println!("Couldn't create mods.zip: {why:?}");
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

    let blacklist_url = match option_env!("MC_BLACKLIST_URL") {
        Some(url) => url,
        None => "https://trae.is/blacklist.txt",
    };
    let response = reqwest::blocking::get(blacklist_url).unwrap();
    let raw_blacklist = response.text().unwrap();
    let blacklist: Vec<&str> = raw_blacklist.split("\n").collect();

    for file in blacklist {
        if !file.is_empty() {
            let file_path = mod_path.join(file);
            if file_path.exists() {
                println!("{} {}", "[-]".red(), file_path.display());
                std::fs::remove_file(file_path)?;
            }
        }
    }

    println!(
        "{} {version} {}",
        "Done! Updated to version".green(),
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
