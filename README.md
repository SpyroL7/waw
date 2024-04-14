# Who's Actually Working
A program to tell you if your collaborators suck or not

### Installation
- Clone the repository (eg. with `git clone https://github.com/SpyroL7/waw.git`)
- Go into the project directory (`cd waw`)
- Run `cargo install --path .`

There are two modes you can run the tool in:
### 1. Config Editing Mode
This mode is enabled by providing a set of flags beginning with '-c' as the first argument. All flags in this mode are lowercase.

(for example: `waw -cra arg1 arg2 arg3` will run the (r)eset function, and then the (a)dd function with the arguments `arg1`, `arg2` and `arg3`)
#### Flags:
- `-a arg1 ... argn`: (a)dds an entry to the config which states when the authors 'args 1-n' appear, group them together under arg1 (arg1 is know as the **alias** of the other names). If the alias arg1 is already in the config, args 2-n are appended to the existing names.
- `-d arg1 ... argn`: (d)eletes the entries in the config file where args 1-n are the aliases (it will let you know if a certain alias was not found).
- `-r`: (r)esets the config file.
- `-p arg`: sets the default project (p)ath to stop you typing it in every time.

Note the order of the arguments is preserved, so `-ar` will result in an empty config whereas `-ra` will not.

### 2. Display Stats Mode
This is the default mode and can be run with no arguments, or will a series of uppercase flags and arguments. Flags which require arguments can be chained.

(for example: `waw -AF feat fix -E bob bill` will use (A)utogenerated aliases, (F)ilter out commits that are not 'feats' or 'fixes' and excludes commits by bob and bill)
#### Flags:
- `-I`: (I)gnores aliases in the config.
- `-P arg`: ignores (P)ath in config, uses arg as repo path instead.
- `-A`: ignores aliases in config and uses an (A)utogenerated config where authors are gotten from each commit message with the format `[author1, author2, ...] _cc_msg: blah blah`, and otherwise marked as 'untagged'.
- `-X`: e(X)clusively uses aliases in config and ignores all other commits.
- `-F arg1 ...`: (F)ilters for commits with a conventional commit message of arg1 or arg2... (case insensitive).
- `-S arg1 ...`: (S)earches for commits with arg1 or arg2... in the body of the commit message (case sensitive).
- `-C arg1...`: (C)ase insensitive version of -S
- `-T arg time_unit`: filters by commits that are from a certain amount of (T)ime ago or sooner - `arg` must be an integer, and `time_unit` can either be `h`, `d`, `w`, `m` or `y` for hours, days, weeks, months (assuming 30 days) or years respectively.

The program will then display a table with the following format, where 'author' is either the commit author's username, an alias, or an autogenerated username depending on the flags set:
`author     | commits    | lines added     | lines deleted   | lines modified per commit | median lines modified`

In the code contains various not quite implemented features and TODOs about what I would like to add. I will most likely not implement these as the project achieved its goal of giving me some statistics about a group project I was working on while teaching me Rust. If I was redoing this, I would probably not separate the tool into two modes, this kind of came about by accident and is confusing - I think I would use a library for handling arguments, or make it an interactive tool. I would also focus more on the statistics side rather than the options and config stuff (I think the alias stuff can be useful in some cases, but I mostly run it raw or with the auto alias setting.
