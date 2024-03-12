use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use std::fs::{OpenOptions, File, remove_file};
use std::io::{self, Write, BufReader, BufRead};


const CONFIG: &str = "config";

fn main() -> Result<(), Error> {
    let mut args: Vec<String> = env::args().skip(1).collect();  // skips the first redundant
                                                               // argument 
    if args[0] == "-r" {
        match reset_config() {
            Ok(_)  => {},
            Err(e) => panic!("Error resetting the config file: {}", e),
        }
        args.remove(0);
    }
    if args.len() > 2 {
        match add_alias(args) {
            Ok(_)  => {},
            Err(e) => panic!("Error occured attempting to add alias: {}", e),
        };
    }

    match get_names_with_alias("joe".to_string()) {
        Ok(_)  => {},
        Err(e) => panic!("Error occured attempting to get names: {}", e),
    };

    let repo = match Repository::open("REO_PATH_HERE") {
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

    let _ = rw.push_head()?;
    for commit in rw.filter_map(|x| x.ok()) {
        let commit_obj = repo.find_commit(commit)?;
        let author_name = match commit_obj.committer().name() { // option str
            Some(name) => name.to_string(),
            None => "ERROR".to_string(),
        };
        *commit_counter.entry(author_name).or_insert(0) += 1;
    }

    println!("{:#?}", commit_counter);

    Ok(())
}

fn add_alias(mut names: Vec<String>) -> Result<(), io::Error> {
    let mut config_file = OpenOptions::new().append(true).create(true).open(CONFIG)?;
    let alias = names.remove(0);
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

fn append_alias(mut names: Vec<String>) -> Result<(), io::Error> {
    let mut config_file = OpenOptions::new().append(true).create(true).open(CONFIG)?;
    let alias = names.remove(0);
    // find line starting with alias, add values to names, remove_alias then add_alias with new
    // names
    Ok(())
}

fn remove_alias(alias: String) -> Result<(), io::Error> {
    Ok(())
}

fn reset_config() -> Result<(), io::Error> {
    remove_file("config")?;
    println!("Config reset");
    Ok(())
}

fn get_names_with_alias(alias: String) -> Result<Vec<String>, io::Error> {
    let mut names: Vec<String> = vec![]; 
    let config_file = File::open(CONFIG)?;
    let mut reader = BufReader::new(config_file);

    for line in reader.lines() {
        let line = line?;
        
        match line.split_once(':') {
            Some((line_alias, ns)) if line_alias == alias  => {
                names = ns.trim().split(", ").map(|n| n.to_string()).collect();
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




