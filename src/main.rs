mod input_handler;
mod consts;
mod config_use;

use input_handler::{process_flags, enable_options};
use consts::*;
use config_use::*;

use git2::{Repository, Error};
use std::collections::HashMap;
use std::env;
use regex::Regex;
use chrono::Local;

// TODO: 'error finding path' when called from anywhere outside group-stats directory

// TODO pivot to interactive tool with a prompt similar to gdb which continuously reads commands so you 
// can change the new config continuously, which would also include the stats you want displayed, and an
// option to 'export' and save in a file at the end of your session
fn main() -> Result<(), Error> {
    let untagged = UNTAGGED.to_string();

    let mut options: Vec<bool> = vec![false; 10];
    let mut args: Vec<String> = env::args().skip(1).collect();  // skips the first redundant argument

    let mut time_seconds: i64 = 0;
    // TODO fix errors so they work properly and panic in the right places
    let mut path = match get_path() {  // try to get path from config
        Ok(path) => path,
        Err(e) => panic!("Error finding path: {}", e),
    };

    let mut arg_vector = vec![vec![]; 5];  // stores option args set with user flags
    let mut first = true;
    let mut flags = String::new();
    
    if args.len() > 0 && args[0].starts_with("-c") {  // config editing mode
        let flags = args.remove(0);
        match process_flags(String::from(&flags[2..]), &mut args) {
            Ok(_) => {},
            Err(e) => panic!("Error editing config file: {}", e),
        };
    } else {  // display data mode
        let current_time = Local::now().timestamp();
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

        let repo = match Repository::open(path) {  // open repo at path provided/in config
            Ok(repo) => repo,
            Err(e) => panic!("Couldn't find repo: {}", e),
        };

        let mut commit_counter: HashMap<String, (usize, usize, usize, Vec<usize>)> = HashMap::new();
        let mut rw = match repo.revwalk() {  // this lets us traverse the commit graph
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

        let pattern = r"\[([^,\]]+)(?:, ([^,\]]+))*\]";  // matches authors when commit message looks like:
        let regex = Regex::new(pattern).unwrap();        // [user1, user2, user3 ...] conv_com_msg: blah blah

        let _ = rw.push_head()?;
        for commit in rw.filter_map(|x| x.ok()) {  // iterate over commit graph with revwalk
            let commit_obj = repo.find_commit(commit)?;
            let parent_commit = match commit_obj.parent(0) {  // parent is needed to use diff to check lines modified since last commit
                Ok(parent) => parent,
                Err(_)  => commit_obj.clone(),  // initial commit has no parent
            };
            // get stats pertaining to changes since last commit
            let stats = repo.diff_tree_to_tree(Some(&parent_commit.tree()?), Some(&commit_obj.tree()?), None)?.stats()?;
            let author_name = match commit_obj.committer().name() {  // get author of current commit
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
            // These boolean values will be true if either a specific option is not enabled, or if the condition is met by this commit
            let filtered = !options[3] || arg_vector[FILTERS].iter().any(|f| data.contains(f));
            let case_insensitive = !options[4] || arg_vector[CI_SEARCH].iter().any(|s| data.to_lowercase().contains(&s.to_lowercase()) || msg.to_lowercase().contains(&s.to_lowercase()));
            let searched = !options[7] || arg_vector[SEARCH].iter().any(|s| data.contains(s) || msg.contains(s));
            let timed = !options[9] || current_time - commit_obj.time().seconds() <= time_seconds;

            if options[5] {  // using autogenerated config with commit message data
                if let Some(captures) = regex.captures(data) {
                    for cap in captures.iter().skip(1) {
                        if let Some(author) = cap {
                            let author_s = author.as_str().to_string();
                            let excluded = !options[6] || !arg_vector[EXCLUDE].contains(&author_s);  // exclude has to be done after the other options so
                            if filtered && case_insensitive && excluded && searched && timed {       // that we can exclude autogenerated names or aliases
                                let counter = commit_counter.entry(author_s).or_insert((0, 0, 0, vec![]));
                                counter.0 += 1;
                                counter.1 += stats.insertions();
                                counter.2 += stats.deletions();
                                counter.3.push(stats.insertions() + stats.deletions());
                            }
                        }
                    }
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
        print_results(commit_counter);
    }

    Ok(())
}

// function to print all the stats I decided you might want in a nice table
fn print_results(commit_counter: HashMap<String, (usize, usize, usize, Vec<usize>)>) {
    // TODO print what filters and such have been used
    // println!("Commits by each user (using/not using config with/without filters, exclusions, searches etc.): \n");
    // TODO only display stats user asks for, add more things
    println!("{:-<115}", "");
    println!( // {arg_no: <char_width}
        "{0: <20} | {1: <10} | {2: <15} | {3: <15} | {4: <25} | {5: <20}",
        "author", "commits", "lines added", "lines deleted", "lines modified per commit", "median lines modified"
    );
    println!("{:-<115}", "");
    for (name, (commits, ins, dels, mut lines)) in commit_counter {
        lines.sort();
        let median = lines[lines.len()/2];  // probably not efficient to store all these but what can you do
        println!("{0: <20} | {1: <10} | {2: <15} | {3: <15} | {4: <25} | {5: <20}", name, commits, ins, dels, (ins+dels)/commits, median);
    }
}
