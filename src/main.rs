use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};
use regex::Regex;

// TODO test case insensitive filtering and name exclusion (with/without config and A)

const CONFIG: &str = "config";

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

fn enable_options(flags: String, options: &mut Vec<bool>) -> () {
    for f in flags.chars() {
        match f {
            'X' => options[0] = true,  // eXclusively use names in config (can't be ysed with I)
            'P' => options[1] = true,  // manually provide Path rather than use one in config
            'I' => options[2] = true,  // Ignore config aliases (can't be used with X)
            'F' => options[3] = true,  // Filter by conventional commit (keyword before first :)
            'C' => options[4] = true,  // makes filter Case insensitive (requires F)
            'A' => options[5] = true,  // use Autopopulated config
            'E' => options[6] = true,  // Exclude name/alias
            bad => println!("Invalid flag/combination: {}", bad),
        }
    };
}

fn main() -> Result<(), Error> {
    let untagged = "untagged".to_string();

    let mut options: Vec<bool> = vec![false; 7];
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
        let mut filters = vec![];
        let mut exclude = vec![];
        if options[1] {
            if args.len() == 0 {  // maybe we should panic here so it doesn't just use saved one
                println!("No path argument provided");
            } else if args.len() == 1 {
                path = args.pop().unwrap();
            } else {
                println!("Too many arguments");
            }
        } else if options[3] || options[4] {
            // TODO find a way to allow multiple flag arguments maybe like -P path -F filter
            filters = args;
        } else if options[6] {
            exclude = args;
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

        let pattern = r"\[([^,\]]+)(?:, ([^,\]]+))*\]";
        let regex = Regex::new(pattern).unwrap();

        let _ = rw.push_head()?;
        for commit in rw.filter_map(|x| x.ok()) {
            let commit_obj = repo.find_commit(commit)?;
            let author_name = match commit_obj.committer().name() { // option str
                Some(name) => name.to_string(),
                None => "ERROR".to_string(),
            };

            let (data, _msg) = match commit_obj.message(){
                Some(commit_msg) => match commit_msg.split_once(":") {
                    Some((data, msg)) => (data, msg),
                    _ => ("", ""),
                },
                None => ("", ""),
            };
            
            let mut found = false;
            let filtered = !options[3] || filters.iter().any(|f| data.contains(f));
            let case_insensitive = !options[4] || filters.iter().any(|f| data.to_lowercase().contains(&f.to_lowercase()));

            if options[5] {
                if let Some(captures) = regex.captures(data) {
                    for cap in captures.iter().skip(1) {
                        if let Some(author) = cap {
                            let author_s = author.as_str().to_string();
                            let excluded = !options[6] || !exclude.contains(&author_s);
                            if (filtered || case_insensitive) && excluded {
                                *commit_counter.entry(author_s).or_insert(0) += 1;
                            }
                        }
                    }
                } else {
                    // choose how to deal with this - maybe ignore or have an unknown
                    if !options[6] || !exclude.contains(&untagged) {
                        *commit_counter.entry(untagged.clone()).or_insert(0) += 1;
                    }
                }
            } else {
                if !options[2] {
                    for (alias, names) in &config_map {
                        let alias_s = alias.to_string();
                        let excluded = !options[6] || !exclude.contains(&alias_s);
                        if names.contains(&author_name) || author_name == *alias {
                            if (filtered && case_insensitive) && excluded {
                                *commit_counter.entry(alias_s).or_insert(0) += 1;
                            }
                            found = true;
                        }
                    };
                }
                if !found && !options[0] {  // author_name is not an alias or in the config
                    let excluded = !options[6] || !exclude.contains(&author_name);
                    if filtered && case_insensitive && excluded {
                        *commit_counter.entry(author_name).or_insert(0) += 1;
                    }
                }
            }
        }

        // TODO: prettier printing!
        println!("{:#?}", commit_counter);
    }

    Ok(())
}

fn add_alias(names: &mut Vec<String>) -> Result<(), io::Error> {
    let alias = names.remove(0);
    match get_names_with_alias(&alias) {
        Ok(result) if result.len() > 0 => { 
            // println!("An entry for '{}' already exists", alias); Ok(())
            *names = vec![result, names.to_vec()].concat();
            let _ = delete_alias(&mut vec![alias.clone()], true)?;
        },
        _ => {}
    }
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


fn delete_alias(aliases: &mut Vec<String>, quietly: bool) -> Result<(), io::Error> {
    let config_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(CONFIG)?;

    let lines = BufReader::new(config_file).lines()
        .map(|x| x.unwrap())
        .filter(|x| {
            match x.split(':').next() {
                Some(name) if aliases.contains(&name.to_string()) => {
                    if !quietly {
                        println!("Deleted entry for '{}'", name);
                    }
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


// fn autogenerate_config() -> Result<(), io::Error> {
//     let path = match get_path() {
//         Ok(path) => path,
//         Err(e) => panic!("Error finding path: {}", e),
//     };
// 
//     let mut rw = match repo.revwalk() {
//         Ok(rw) => rw,
//         Err(e) => {
//             println!("Error creating revwalk: {}", e);
//             return Err(e.into());
//         }
//     };
// 
//     // TODO put this all into main loop instead of seperate copy
//     let pattern = r"[\s*[(.),*]*\s*]";
//     let regex = Regex::new(pattern).unwrap();
// 
//     let _ = rw.push_head()?;
//     for commit in rw.filter_map(|x| x.ok()) {
//         let commit_obj = repo.find_commit(commit)?;
//         let author_name = match commit_obj.committer().name() { // option str
//             Some(name) => name.to_string(),
//             None => "ERROR".to_string(),
//         };
// 
//         let (data, _msg) = match commit_obj.message(){
//             Some(commit_msg) => match commit_msg.split_once(":") {
//                 Some((data, msg)) => (data, msg),
//                 _ => ("", ""),
//             },
//             None => ("", ""),
//         };
// 
//         if let Some(captures) = regex.captures(data) {
//             let mut done = false;
//             let mut i = 0;
//             while !done {
//                 match captures.get(i) {
//                     Some(author) => (), // add 1 to counter here
//                     None => done = true,
//                 }
//                 i += 1;
//             }
//         } else {
//             // choose how to deal with this - maybe ignore or have an unknown
//         }
//         if !options[2] {
//             for (alias, names) in &config_map {
//                 if names.contains(&author_name) || author_name == *alias {
//                     // TODO maybe add option to make filter non case sensitive
//                     if !options[3] || filters.iter().any(|f| data.contains(f)) {
//                         *commit_counter.entry(alias.to_string()).or_insert(0) += 1;
//                     }
//                     found = true;
//                 }
//             };
//         }
//         if !found && !options[0] {  // author_name is not an alias or in the confige
//             if !options[3] || filters.iter().any(|f| data.contains(f)) {
//                 *commit_counter.entry(author_name).or_insert(0) += 1;
//             }
//         }
//     }
// 
//     Ok(())
// }


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
        match path_line.split(' ').nth(1) {
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
                    // fs::write(CONFIG, to_write)?;
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

