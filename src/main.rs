use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Episode {
    path: PathBuf,
    episode_number: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Program {
    name: String,
    hash: String,
    directory: PathBuf,
    episodes: Vec<Episode>,
    current_episode: usize,
    start_date: Option<DateTime<Utc>>,
    last_update: Option<DateTime<Utc>>,
    status: ProgramStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum ProgramStatus {
    Ready,
    Running,
    Finished,
    Stopped,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Database {
    programs: HashMap<String, Program>,
    symlink_dir: Option<PathBuf>,
}

impl Database {
    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| "Could not find home directory")?;
        
        let config_dir = Path::new(&home_dir).join(".config").join("queuecast");
        fs::create_dir_all(&config_dir)?;
        
        Ok(config_dir.join("queuecast.json"))
    }

    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Database {
                programs: HashMap::new(),
                symlink_dir: None,
            })
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }
}

fn generate_hash(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..8].to_string()
}

fn scan_episodes(dir: &Path) -> Result<Vec<Episode>, Box<dyn std::error::Error>> {
    let mut episodes = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().is_file() && 
            entry.path().extension().map_or(false, |ext| {
                matches!(ext.to_str(), Some("mp4") | Some("mkv") | Some("avi") | Some("mov"))
            })
        })
        .collect();
    
    entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    
    for (i, entry) in entries.iter().enumerate() {
        episodes.push(Episode {
            path: entry.path(),
            episode_number: i + 1,
        });
    }
    
    Ok(episodes)
}

fn add_program(db: &mut Database, directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = PathBuf::from(directory);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err("Directory does not exist or is not a directory".into());
    }

    let name = dir_path.file_name()
        .ok_or("Invalid directory name")?
        .to_string_lossy()
        .to_string();
    
    let hash = generate_hash(&name);
    let episodes = scan_episodes(&dir_path)?;
    
    if episodes.is_empty() {
        return Err("No video files found in directory".into());
    }

    let program = Program {
        name: name.clone(),
        hash: hash.clone(),
        directory: dir_path,
        episodes,
        current_episode: 0,
        start_date: None,
        last_update: None,
        status: ProgramStatus::Ready,
    };

    db.programs.insert(hash.clone(), program);
    println!("Added program '{}' with hash '{}'", name, hash);
    Ok(())
}

fn list_programs(db: &Database, filter: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status_filter = match filter {
        "running" => Some(ProgramStatus::Running),
        "ran" => Some(ProgramStatus::Finished),
        "ready" => Some(ProgramStatus::Ready),
        "stopped" => Some(ProgramStatus::Stopped),
        _ => None,
    };

    for program in db.programs.values() {
        if let Some(ref filter_status) = status_filter {
            if &program.status != filter_status {
                continue;
            }
        }
        
        println!("{} [{}] ({}/{} episodes) - {:?}", 
            program.hash, 
            program.name,
            program.current_episode,
            program.episodes.len(),
            program.status
        );
    }
    Ok(())
}

fn should_rollover(last_update: Option<DateTime<Utc>>) -> bool {
    match last_update {
        None => true, // First time, always rollover
        Some(last) => {
            let now = Utc::now();
            let days_since = now.signed_duration_since(last).num_days();
            days_since >= 7
        }
    }
}

fn update_program_symlink(db: &mut Database, program_hash: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let symlink_dir = db.symlink_dir.as_ref()
        .ok_or("Symlink directory not configured. Use 'queuecast config symlink-dir <path>' to set it.")?;

    let program = db.programs.get_mut(program_hash)
        .ok_or("Program not found")?;

    // Create symlink directory if it doesn't exist
    fs::create_dir_all(symlink_dir)?;

    // Start the program if it's ready
    if program.status == ProgramStatus::Ready {
        program.status = ProgramStatus::Running;
        program.start_date = Some(Utc::now());
    }

    if program.status != ProgramStatus::Running {
        return Ok(()); // Skip non-running programs
    }

    // Check if we should rollover to next episode
    if !force && !should_rollover(program.last_update) {
        return Ok(()); // Not time to rollover yet
    }

    // Check if we have more episodes
    if program.current_episode >= program.episodes.len() {
        program.status = ProgramStatus::Finished;
        return Ok(());
    }

    let episode = &program.episodes[program.current_episode];
    let symlink_path = symlink_dir.join(format!("{}_ep{:02}.{}", 
        program.name.replace(" ", "_"),
        episode.episode_number,
        episode.path.extension().unwrap_or_default().to_string_lossy()
    ));

    // Remove existing symlink if it exists
    if symlink_path.exists() {
        fs::remove_file(&symlink_path)?;
    }

    // Create new symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&episode.path, &symlink_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&episode.path, &symlink_path)?;
    
    println!("Created symlink for {} episode {}", program.name, episode.episode_number);
    
    // Advance to next episode and update timestamp
    program.current_episode += 1;
    program.last_update = Some(Utc::now());
    
    Ok(())
}

fn update_symlinks(db: &mut Database, program_hash: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    match program_hash {
        Some(hash) => {
            // Update specific program, force rollover
            update_program_symlink(db, hash, true)?;
        }
        None => {
            // Update all running programs, respect weekly schedule
            let program_hashes: Vec<String> = db.programs.keys().cloned().collect();
            for hash in program_hashes {
                if let Err(e) = update_program_symlink(db, &hash, false) {
                    eprintln!("Error updating program {}: {}", hash, e);
                }
            }
        }
    }
    Ok(())
}

fn remove_program(db: &mut Database, program_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(program) = db.programs.remove(program_hash) {
        println!("Removed program '{}'", program.name);
    } else {
        return Err("Program not found".into());
    }
    Ok(())
}

fn stop_program(db: &mut Database, program_hash: &str) -> Result<(), Box<dyn std::error::Error>> {
    let program = db.programs.get_mut(program_hash)
        .ok_or("Program not found")?;
    
    program.status = ProgramStatus::Stopped;
    println!("Stopped program '{}'", program.name);
    Ok(())
}

fn set_symlink_dir(db: &mut Database, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = PathBuf::from(path);
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&dir_path)?;
    
    db.symlink_dir = Some(dir_path.clone());
    println!("Set symlink directory to: {}", dir_path.display());
    Ok(())
}

fn skip_episodes(db: &mut Database, program_hash: &str, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let program = db.programs.get_mut(program_hash)
        .ok_or("Program not found")?;
    
    program.current_episode = (program.current_episode + count).min(program.episodes.len());
    println!("Skipped {} episodes for program '{}'", count, program.name);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("queuecast")
        .version("0.1.0")
        .about("Manage TV show files with weekly scheduling")
        .subcommand(
            Command::new("add")
                .about("Add directory to database")
                .arg(Arg::new("directory").required(true))
        )
        .subcommand(
            Command::new("list")
                .about("List programs")
                .arg(Arg::new("filter").value_parser(["running", "ran", "ready", "stopped"]))
        )
        .subcommand(
            Command::new("update")
                .about("Update symlinks for programs (all programs by default, or specific program)")
                .arg(Arg::new("program").required(false))
        )
        .subcommand(
            Command::new("remove")
                .about("Remove program from database")
                .arg(Arg::new("program").required(true))
        )
        .subcommand(
            Command::new("stop")
                .about("Stop program from broadcasting")
                .arg(Arg::new("program").required(true))
        )
        .subcommand(
            Command::new("skip")
                .about("Skip episodes")
                .arg(Arg::new("program").required(true))
                .arg(Arg::new("count").value_parser(clap::value_parser!(usize)).default_value("1"))
        )
        .subcommand(
            Command::new("config")
                .about("Configure settings")
                .subcommand(
                    Command::new("symlink-dir")
                        .about("Set the symlink directory")
                        .arg(Arg::new("path").required(true))
                )
        )
        .get_matches();

    let mut db = Database::load()?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let directory = sub_matches.get_one::<String>("directory").unwrap();
            add_program(&mut db, directory)?;
        }
        Some(("list", sub_matches)) => {
            let filter = sub_matches.get_one::<String>("filter").map(|s| s.as_str()).unwrap_or("all");
            list_programs(&db, filter)?;
        }
        Some(("update", sub_matches)) => {
            let program = sub_matches.get_one::<String>("program").map(|s| s.as_str());
            update_symlinks(&mut db, program)?;
        }
        Some(("remove", sub_matches)) => {
            let program = sub_matches.get_one::<String>("program").unwrap();
            remove_program(&mut db, program)?;
        }
        Some(("stop", sub_matches)) => {
            let program = sub_matches.get_one::<String>("program").unwrap();
            stop_program(&mut db, program)?;
        }
        Some(("skip", sub_matches)) => {
            let program = sub_matches.get_one::<String>("program").unwrap();
            let count = *sub_matches.get_one::<usize>("count").unwrap();
            skip_episodes(&mut db, program, count)?;
        }
        Some(("config", sub_matches)) => {
            match sub_matches.subcommand() {
                Some(("symlink-dir", config_matches)) => {
                    let path = config_matches.get_one::<String>("path").unwrap();
                    set_symlink_dir(&mut db, path)?;
                }
                _ => {
                    println!("Use 'queuecast config --help' for configuration options");
                }
            }
        }
        _ => {
            println!("Use --help for usage information");
        }
    }

    db.save()?;
    Ok(())
}
