use crate::consts::CONFIG;

use std::collections::HashMap;
use std::fs::{self, OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};

// adds an alias with a list of matching names in the form 'alias: name1, name2, ... \n'
pub fn add_alias(names: &mut Vec<String>) -> Result<(), io::Error> {
    let alias = names.remove(0);          // get the first name in the list of args
    match get_names_with_alias(&alias) {  // if it is already in the config, new entries to existing ones
        Ok(result) if result.len() > 0 => { 
            *names = vec![result, names.to_vec()].concat();
            let _ = delete_alias(&mut vec![alias.clone()], true)?;
        },
        _ => {}
    }
    let mut config_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(CONFIG)?;
    config_file.write_all(format!("{}: ", alias).as_bytes())?;  // write first name

    let mut it = names.iter().peekable();
    while let Some(name) = it.next() {  // write all the other names
         if it.peek().is_none() {
            config_file.write_all(format!("{}\n", name).as_bytes())?;
        } else {
            config_file.write_all(format!("{}, ", name).as_bytes())?;
        }
    }

    Ok(())
}

// deletes alias entries from config
pub fn delete_alias(aliases: &mut Vec<String>, quietly: bool) -> Result<(), io::Error> {
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(CONFIG)?;

    let lines = BufReader::new(config_file).lines()
        .map(|x| x.unwrap())
        .filter(|x| {  // filter out lines to be deleted, then rewrite existing data to blank config
            match x.split(':').next() {
                Some(name) if aliases.contains(&name.to_string()) => {
                    if !quietly {  // only print if called by user
                        println!("Deleted entry for '{}'", name);
                    }
                    false
                },
                _ => true  // also covers us for non-alias lines such as the path
            }
        })
        .collect::<Vec<String>>()
        .join("\n") + "\n";

    fs::write(CONFIG, lines)?;
    
    Ok(())
}

// deletes the config file (resetting it)
pub fn reset_config() -> Result<(), io::Error> {
    remove_file("config")?;    
    println!("Config reset");  // next time we add it will make a new one

    Ok(())
}

pub fn get_map() -> Result<HashMap<String, Vec<String>>, io::Error> { 
    let config_file = OpenOptions::new()  // turns config into usable map
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

// get the names from a single row matching the given alias
pub fn get_names_with_alias(alias: &String) -> Result<Vec<String>, io::Error> {
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
            }
            _ => (),
        }
    }

    Ok(names)
}

// get repository path from config file
pub fn get_path() -> Result<String, io::Error> {
    let config_file = File::open(CONFIG)?;
    let mut lines = BufReader::new(config_file).lines();
    if let Some(Ok(path_line)) = lines.next() {
        match path_line.split(' ').nth(1) {
            None => println!("No path found in config file"),
            Some(saved_path) => return Ok(saved_path.to_string()),
        }
    } else {
        println!("No path found in config file");
    }

    Ok(String::new())
}

// save default path of repository to config
pub fn set_path(args: &mut Vec<String>) -> Result<(), io::Error> {
    match args.len() {
        0 => println!("Provide a path to add to the config"),
        1 => {
            let path = format!("# {}\n", args[0]);
            let config_file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(CONFIG)?;

            let mut lines = BufReader::new(config_file).lines();
            if let Some(Ok(path_line)) = lines.next() {
                if path_line.starts_with("# ") {
                    let to_write = lines
                        .map(|x| x.unwrap())
                        .collect::<Vec<String>>().join("\n");

                    fs::write(CONFIG, path + &to_write)?;
                }
            } else {
                fs::write(CONFIG, path)?;
            }
        }
        _ => println!("Too many arguments provided"),
    }

    Ok(())
}