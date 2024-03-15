use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};


const CONFIG: &str = "config";

// TODO add flag to use [user] instead of config or to autopopulate
// TODO add ignore names like LTS to config
// TODO flag that ignores names not in config
fn process_flags(flags: String, args: &mut Vec<String>) -> Result<(), io::Error> {
    let mut used_args = false;
    for f in flags.chars() {
         match f {
            'r' => reset_config()?,
            'd' if !used_args => { used_args = true; delete_alias(args)? },
            'a' if !used_args => { used_args = true; add_alias(args)? },
            'e' if !used_args => { used_args = true; extend_alias(args)? },
            'p' if !used_args => { used_args = true; set_path(args)? },
            bad  => println!("Invalid flag/combination: {}", bad),
        };
    };
    Ok(())
}

fn enable_options(flags: String, options: &mut Vec<bool>) -> () {
    for f in flags.chars() {
        match f {
            'X' => options[0] = true,  // eXclusively use names in config
            'P' => options[1] = true,  // manually provide Path rather than use one in config
            'I' => options[2] = true,  // Ignore config aliases
            bad => println!("Invalid flag/combination: {}", bad),
        }
    };
}

fn main() -> Result<(), Error> {
    let mut options: Vec<bool> = vec![false; 5];
    let mut args: Vec<String> = env::args().skip(1).collect();  // skips the first redundant
                                                                // argument 

    if args.len() > 0 && args[0].starts_with("-c") {
        let flags = args.remove(0);
        match process_flags(String::from(&flags[2..]), &mut args) {
            Ok(_) => {},
            Err(e) => panic!("Error editing config file: {}", e),
        };
    } else {
        // TODO put this is a 'display stats' function
        if args.len() > 0 && args[0].starts_with("-") {
            let flags = args.remove(0);
            enable_options(String::from(&flags[1..]), &mut options);
        }

        let mut path = match get_path() {
            Ok(path) => path,
            Err(e) => panic!("Error finding path: {}", e),
        };
        if options[1] {
            if args.len() == 0 {  // maybe we should panic here so you dont also get repo not
                                     // found error?
                println!("No path argument provided");
            } else if args.len() == 1 {
                path = args.pop().unwrap();
            } else {
                println!("Too many arguments");
            }
        }

        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(e) => panic!("Couldn't find repo: {}", e),
        };

        let mut commit_counter: HashMap<String, usize> = HashMap::new();
        let mut rw = match repo.revwalk() {
            Ok(rw) => rw,
            Err(e) => {
                println!("Error creating revwalk: {}", e);
                return Err(e.into());
             }
        };

        let config_map = match get_map() {
            Ok(config_map) => config_map,
            Err(e) => panic!("Couldn't parse config file: {}", e),
        };

        // TODO filter by conventional commit message (eg. only Feats)
        let _ = rw.push_head()?;
        for commit in rw.filter_map(|x| x.ok()) {
            let commit_obj = repo.find_commit(commit)?;
            let author_name = match commit_obj.committer().name() { // option str
                Some(name) => name.to_string(),
                None => "ERROR".to_string(),
            };
            let mut found = false;
            if !options[2] {
                for (alias, names) in &config_map {
                    if names.contains(&author_name) || author_name == *alias {
                        *commit_counter.entry(alias.to_string()).or_insert(0) += 1;
                        found = true;
                    }
                };
            }
            if !found && !options[0] {  // author_name is not an alias or in the confige
                *commit_counter.entry(author_name).or_insert(0) += 1;
            }
        }

        // TODO: prettier printing!
        println!("{:#?}", commit_counter);
    }

    Ok(())
}

// TODO: combine add and extend for simplicity
fn add_alias(names: &mut Vec<String>) -> Result<(), io::Error> {
    let alias = names.remove(0);
    match get_names_with_alias(&alias) {
        Ok(result) if result.len() > 0 => { println!("An entry for '{}' already exists", alias); Ok(()) },
        _ => {
            let mut config_file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(CONFIG)?;
            config_file.write_all(format!("{}: ", alias).as_bytes())?;

            let mut it = names.iter().peekable();
            while let Some(name) = it.next() {
                if it.peek().is_none() {
                    config_file.write_all(format!("{}\n", name).as_bytes())?;
                } else {
                  config_file.write_all(format!("{}, ", name).as_bytes())?;
                }
            }
            Ok(())
        }
    }
}

// TODO add function to save repo path in config, another to accept path as argument

fn extend_alias(names: &mut Vec<String>) -> Result<(), io::Error> {
    let alias = names.remove(0);
    match get_names_with_alias(&alias) {
        Ok(result) if result.len() ==  0 => { println!("No alias '{}' found to extend", alias); Ok(()) },
        _ => {
            let mut _config_file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(CONFIG)?;

    // find line starting with alias, add values to names, remove_alias then add_alias with new
    // names
            Ok(())
        }
    }
}

fn delete_alias(aliases: &mut Vec<String>) -> Result<(), io::Error> {
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(CONFIG)?;

    let lines = BufReader::new(config_file).lines()
        .map(|x| x.unwrap())
        .filter(|x| {
            match x.split(':').next() {
                Some(name) if aliases.contains(&name.to_string()) => {
                    println!("Deleted entry for '{}'", name);
                    false
                },
                _ => true
            }
        })
        .collect::<Vec<String>>()
        .join("\n") + "\n";

    fs::write(CONFIG, lines)?;
    
    Ok(())
}

fn reset_config() -> Result<(), io::Error> {
    remove_file("config")?;
    println!("Config reset");
    Ok(())
}

fn get_map() -> Result<HashMap<String, Vec<String>>, io::Error> { 
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(CONFIG)?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for line in BufReader::new(config_file).lines() {
        let line = line?;
        if !line.starts_with("# ") {
            match line.split_once(':') {
                Some((alias, ns)) => {
                    let names = ns.trim()
                        .split(", ")
                        .map(|n| n.to_string())
                        .collect();
                    map.insert(alias.to_string(), names);
                }
                _ => ()
            }
        }
    }

    Ok(map)
}


// fn get_aliases() -> Result<Vec<String>, io::Error> {
//     let mut config_file = OpenOptions::new().append(true).create(true).open(CONFIG)?;
//     Ok(())
// }

fn get_names_with_alias(alias: &String) -> Result<Vec<String>, io::Error> {
    let mut names: Vec<String> = vec![]; 
    let config_file = File::open(CONFIG)?;
    let reader = BufReader::new(config_file);

    for line in reader.lines() {
        let line = line?;
        
        match line.split_once(':') {
            Some((line_alias, ns)) if line_alias == alias  => {
                names = ns.trim()
                    .split(", ")
                    .map(|n| n.to_string())
                    .collect();
                // println!("{:#?}", names);
            }
            _ => (),
        }
    }

    Ok(names)
}

fn get_path() -> Result<String, io::Error> {
    let config_file = File::open(CONFIG)?;
    let mut lines = BufReader::new(config_file).lines();
    if let Some(Ok(path_line)) = lines.next() {
        match path_line.split(' ').next() {
            None => println!("No path found in config file"),
            Some(saved_path) => return Ok(saved_path.to_string()),
        }
    } else {
        println!("No path found in config file");
    }
    Ok("".to_string())
}

fn set_path(args: &mut Vec<String>) -> Result<(), io::Error> {
    match args.len() {
        0 => println!("Provide a path to add to the config"),
        1 => {
            // check if path exists, if so delete it
            // write new path to top starting with # 

            let path = format!("# {}", args[0]);
            let config_file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(CONFIG)?;

            let mut lines = BufReader::new(config_file).lines();
            if let Some(Ok(path_line)) = lines.next() {
                if path_line.starts_with("# ") {
                    let to_write = lines.skip(1)
                        .map(|x| x.unwrap())
                        .collect::<Vec<String>>().join("\n");

                    fs::write(CONFIG, path)?;
                    fs::write(CONFIG, to_write)?;
                }
            } else {
                fs::write(CONFIG, path)?;
            }
        }
        _ => println!("Too many arguments provided"),
    }

    Ok(())
}

// fn create_or_open(filename: &str) -> Result<File, io::Error> {
//     OpenOptions::new().append(true).create(true).open(filename.to_string())
// }

