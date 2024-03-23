use crate::consts::*;
use crate::config_use::*;

use std::io;

// calls functions to edit the config
pub fn process_flags(flags: String, args: &mut Vec<String>) -> Result<(), io::Error> {
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
pub fn enable_options(flags: String, mut new_args: Vec<String>, options: &mut Vec<bool>, path: &mut String, time_seconds: &mut i64, arg_vector: &mut Vec<Vec<String>>) -> () {
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