use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};


const CONFIG: &str = "config";


fn process_flags(flags: String, args: &mut Vec<String>) -> Result<(), io::Error> {
    let mut used_args = false;
    for f in flags.chars() {
         match f {
            'r' => reset_config()?,
            'd' if !used_args => { used_args = true; delete_alias(args)? },
            'a' if !used_args => { used_args = true; add_alias(args)? },
            'e' if !used_args => { used_args = true; extend_alias(args)? },
            bad  => println!("Invalid flag/combination: {}", bad),
        };
    };
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut args: Vec<String> = env::args().skip(1).collect();  // skips the first redundant
                                                               // argument 

    if args.len() > 0 && args[0].chars().next().unwrap() == '-' {
        let flags = args.remove(0);
        match process_flags(String::from(&flags[1..]), &mut args) {
            Ok(_) => {},
            Err(e) => panic!("Error editing config file: {}", e),
        };
    } else {
        // TODO put this is a 'display stats' function
        let repo = match Repository::open("REPO_PATH_HERE") {
            Ok(repo) => repo,
            Err(e) => panic!("Couldn't find repo: {}", e),
        };

        // TODO use the aliases in the config for the counter
        let mut commit_counter: HashMap<String, usize> = HashMap::new();
        let mut rw = match repo.revwalk() {
            Ok(rw) => rw,
            Err(e) => {
                println!("Error creating revwalk: {}", e);
                return Err(e.into());
             }
        };

        let _ = rw.push_head()?;
        for commit in rw.filter_map(|x| x.ok()) {
            let commit_obj = repo.find_commit(commit)?;
            let author_name = match commit_obj.committer().name() { // option str
                Some(name) => name.to_string(),
                None => "ERROR".to_string(),
            };
            *commit_counter.entry(author_name).or_insert(0) += 1;
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
            let mut config_file = OpenOptions::new().append(true).create(true).open(CONFIG)?;
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

// fn create_or_open(filename: &str) -> Result<File, io::Error> {
//     OpenOptions::new().append(true).create(true).open(filename.to_string())
// }




