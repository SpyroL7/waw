use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};
use regex::Regex;
use chrono::Local;


const CONFIG: &str = "config";

const FILTERS   : usize = 0;
const EXCLUDE   : usize = 1;
const SEARCH    : usize = 2;
const CI_SEARCH : usize = 3;
const BRANCHES  : usize = 4;

const HOURS     : i64 = 60*60;
const DAYS      : i64 = 24*HOURS;
const WEEKS     : i64 = 7*DAYS;
const MONTHS    : i64 = 30*DAYS;
const YEARS     : i64 = 52*WEEKS;


// calls functions to edit the config
fn process_flags(flags: String, args: &mut Vec<String>) -> Result<(), io::Error> {
    let mut used_args = false;
    for f in flags.chars() {
         match f {
            'r' => reset_config()?,
            'g' => (),  // autogenerate_config()?,
            'd' if !used_args => { used_args = true; delete_alias(args, false)? },
            'a' if !used_args => { used_args = true; add_alias(args)? },
            'p' if !used_args => { used_args = true; set_path(args)? },
            bad  => println!("Invalid flag/combination: {}", bad),
        };
    };

    Ok(())
}

// sets option flags and assigns the correct arguments to the relevant variables
fn enable_options(flags: String, mut new_args: Vec<String>, options: &mut Vec<bool>, path: &mut String, time_seconds: &mut i64, arg_vector: &mut Vec<Vec<String>>) -> () {
    let mut used_args = false;
    for f in flags.chars() {
        match f {
            'X' => options[0] = true,  // eXclusively use names in config (can't be ysed with I)
            'I' => options[2] = true,  // Ignore config aliases (can't be used with X)
            'A' => options[5] = true,  // use Autopopulated config

            'P' if !used_args => {  // manually provide Path rather than use one in config
                used_args = true; 
                options[1] = true;
                if new_args.len() == 0 {  // maybe we should panic here so it doesn't just use saved one
                    println!("No path argument provided");
                } else if new_args.len() == 1 {
                    *path = new_args.pop().unwrap();
                } else {
                    println!("Too many arguments");
                }
            },
            'F' if !used_args => {  // Filter by conventional commit (keyword before first :)
                used_args = true; 
                options[3] = true;
                arg_vector.get_mut(FILTERS).unwrap().append(&mut new_args.clone());
            },
            'E' if !used_args => {  // Exclude name/alias
                used_args = true; 
                options[6] = true;
                arg_vector.get_mut(EXCLUDE).unwrap().append(&mut new_args.clone());
            },
            'S' if !used_args => {  // Search commit message
                used_args = true;
                options[7] = true;
                arg_vector.get_mut(SEARCH).unwrap().append(&mut new_args.clone());
            },
            'C' if !used_args => {  // Case insensitive search
                used_args = true; 
                options[4] = true;
                arg_vector.get_mut(CI_SEARCH).unwrap().append(&mut new_args.clone());
            },
            'B' if !used_args => {  // filter by specific Branches
                used_args = true;   // this is actually really hard so maybe not
                options[8] = true;
                arg_vector.get_mut(BRANCHES).unwrap().append(&mut new_args.clone());
            },
            'T' if !used_args => {  // only count commits in the last x hours/days/weeks/months
                used_args = true;
                options[9] = true;
                if new_args.len() != 2 {
                    println!("enter number followed by time unit ([h]ours, [d]ays, [w]eeks, [m]onths or [y]ears)");
                } else {
                    let units = match new_args[1].as_str() {
                        "h" => HOURS,
                        "d" => DAYS,
                        "w" => WEEKS,
                        "m" => MONTHS,
                        "y" => YEARS,
                        _   => { println!("Enter a valid time unit"); 0 },
                    };
                    *time_seconds = new_args[0].parse::<i64>().expect("Failed to parse string to int") * units
                }
            },

            // TODO add 'S' search which is like filter but for the commit message rather than the 'data', make filter case insensitive by default and make C and ci version of search
            // TODO add 'B' to select a certain branch you want to filter commits for
            bad => panic!("Invalid option/combination: {}", bad),
        }
    };
}

fn main() -> Result<(), Error> {
    let untagged = "untagged".to_string();

    let mut options: Vec<bool> = vec![false; 10];
    let mut args: Vec<String> = env::args().skip(1).collect();  // skips the first redundant argument

    let mut path = match get_path() {  // try to get path from config
        Ok(path) => path,
        Err(e) => panic!("Error finding path: {}", e),
    };

    let mut arg_vector = vec![vec![]; 5];
    let mut first = true;
    let mut flags = String::new();
    let mut time_seconds: i64 = 0;

    if args.len() > 0 && args[0].starts_with("-c") {  // config editing mode
        let flags = args.remove(0);
        match process_flags(String::from(&flags[2..]), &mut args) {
            Ok(_) => {},
            Err(e) => panic!("Error editing config file: {}", e),
        };
    } else {  // display data mode
        let current_time = Local::now().timestamp();
        // TODO put this is a 'display stats' function
        let mut new_args = vec![];
        for arg in &args {  // find flags, make list of arguments, process and repeat
            if !arg.starts_with("-") {
                new_args.push(arg.to_string());
            } else {
                if !first {  // gets the flag(s) before the list of arguments when a new flag is encountered
                    enable_options(flags, new_args, &mut options, &mut path, &mut time_seconds, &mut arg_vector);
                    new_args = vec![];
                } else {
                    first = false;
                }
                flags = String::from(&arg[1..]);
            }
        }
        if !first {  // use final flag when we run out of arguments
            enable_options(flags, new_args, &mut options, &mut path, &mut time_seconds, &mut arg_vector);
        }

        let repo = match Repository::open(path) {
            Ok(repo) => repo,
            Err(e) => panic!("Couldn't find repo: {}", e),
        };

        let mut commit_counter: HashMap<String, (usize, usize, usize, Vec<usize>)> = HashMap::new();
        let mut rw = match repo.revwalk() {
            Ok(rw) => rw,
            Err(e) => {
                println!("Error creating revwalk: {}", e);
                return Err(e.into());
             }
        };

        let config_map = match get_map() {  // reads config file and puts data in map
            Ok(config_map) => config_map,
            Err(e) => panic!("Couldn't parse config file: {}", e),
        };

        let pattern = r"\[([^,\]]+)(?:, ([^,\]]+))*\]";
        let regex = Regex::new(pattern).unwrap();

        let _ = rw.push_head()?;
        for commit in rw.filter_map(|x| x.ok()) {  // iterate over commit graph with revwalk
            // println!("{:?}", get_branch_name(commit));
            let commit_obj = repo.find_commit(commit)?;
            let parent_commit = match commit_obj.parent(0) {
                Ok(parent) => parent,
                Err(_)  => commit_obj.clone(),
            };
            let stats = repo.diff_tree_to_tree(Some(&parent_commit.tree()?), Some(&commit_obj.tree()?), None)?.stats()?;
            let author_name = match commit_obj.committer().name() {
                Some(name) => name.to_string(),
                None => "ERROR".to_string(),
            };

            let (data, msg) = match commit_obj.message() {  // split at colon to get 'data' (type of commit and contributors)
                Some(commit_msg) => match commit_msg.split_once(":") {
                    Some((data, msg)) => (data, msg),
                    _ => ("", commit_msg),
                },
                None => ("", ""),  // probably shouldn't end up here even if there is no colon
            };
            
            let mut found = false;
            let filtered = !options[3] || arg_vector[FILTERS].iter().any(|f| data.contains(f));
            let case_insensitive = !options[4] || arg_vector[CI_SEARCH].iter().any(|s| data.to_lowercase().contains(&s.to_lowercase()) || msg.to_lowercase().contains(&s.to_lowercase()));
            let searched = !options[7] || arg_vector[SEARCH].iter().any(|s| data.contains(s) || msg.contains(s));
            let timed = !options[9] || current_time - commit_obj.time().seconds() <= time_seconds;
            // println!("dif(days): {}", (current_time - commit_obj.time().seconds()) / (60*60*24));

            if options[5] {  // using autogenerated config with commit message data
                if let Some(captures) = regex.captures(data) {
                    for cap in captures.iter().skip(1) {
                        if let Some(author) = cap {
                            let author_s = author.as_str().to_string();
                            let excluded = !options[6] || !arg_vector[EXCLUDE].contains(&author_s);
                            if filtered && case_insensitive && excluded && searched && timed {
                                let counter = commit_counter.entry(author_s).or_insert((0, 0, 0, vec![]));
                                counter.0 += 1;
                                counter.1 += stats.insertions();
                                counter.2 += stats.deletions();
                                counter.3.push(stats.insertions() + stats.deletions());
                            }
                        }
                    }  // TODO: make filter case insensitive
                } else {  // no contributors listed in the expected format
                    // choose how to deal with this - maybe ignore or have an unknown
                    if !options[6] || !arg_vector[EXCLUDE].contains(&untagged) {
                        let counter = commit_counter.entry(untagged.clone()).or_insert((0, 0, 0, vec![]));
                        counter.0 += 1;
                        counter.1 += stats.insertions();
                        counter.2 += stats.deletions();
                        counter.3.push(stats.insertions() + stats.deletions());
                    }
                }
            } else {
                if !options[2] {  // only do this if we are not ignorning the config
                    for (alias, names) in &config_map {
                        let alias_s = alias.to_string();
                        let excluded = !options[6] || !arg_vector[EXCLUDE].contains(&alias_s);
                        if names.contains(&author_name) || author_name == *alias {
                            if filtered && case_insensitive && excluded && searched && timed {
                                let counter = commit_counter.entry(alias_s).or_insert((0, 0, 0, vec![]));
                                counter.0 += 1;
                                counter.1 += stats.insertions();
                                counter.2 += stats.deletions();
                                counter.3.push(stats.insertions() + stats.deletions());
                            }
                            found = true;
                        }
                    };
                }
                if !found && !options[0] {  // author_name is not an alias or in the config (or we ignored config)
                    let excluded = !options[6] || !arg_vector[EXCLUDE].contains(&author_name);
                    if filtered && case_insensitive && excluded && searched && timed {
                        let counter = commit_counter.entry(author_name).or_insert((0, 0, 0, vec![]));
                        counter.0 += 1;
                        counter.1 += stats.insertions();
                        counter.2 += stats.deletions();
                        counter.3.push(stats.insertions() + stats.deletions());
                    }
                }
            }
        }

        // TODO: prettier printing!
        // println!("{:#?}", commit_counter);
        // TODO median commit length: average is heavily skewed by adding test files and such
        println!("Commits by each user (using/not using config with/without filters, exclusions, searches etc.): \n");
        for (name, (commits, ins, dels, mut lines)) in commit_counter {
            lines.sort();
            let median = lines[lines.len()/2];
            println!("{}:  {}  {}  {}  {}, {}", name, commits, ins, dels, (ins+dels)/commits, median);
        }
    }

    Ok(())
}

// adds an alias with a list of matching names in the form 'alias: name1, name2, ... \n'
fn add_alias(names: &mut Vec<String>) -> Result<(), io::Error> {
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
fn delete_alias(aliases: &mut Vec<String>, quietly: bool) -> Result<(), io::Error> {
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
fn reset_config() -> Result<(), io::Error> {
    remove_file("config")?;    
    println!("Config reset");  // next time we add it will make a new one

    Ok(())
}

fn get_map() -> Result<HashMap<String, Vec<String>>, io::Error> { 
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
            }
            _ => (),
        }
    }

    Ok(names)
}

// get repository path from config file
fn get_path() -> Result<String, io::Error> {
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
fn set_path(args: &mut Vec<String>) -> Result<(), io::Error> {
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



