use git2::{Repository, Error};

use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let _ = match add_alias(args[1..].to_vec()) {
        Ok(a) => a,
        Err(e)  => panic!("Error occured attempting to add alias: {}", e),
    };
    let repo = match Repository::open("REPO_PATH_HERE") {
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
    // let cs = rw.count();
   //  println!("{}", cs);
    // rw = repo.revwalk()?;
    // println!("commits: {}", cs);
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
    let mut config_file = OpenOptions::new().append(true).create(true).open("config")?;
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




