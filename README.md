# amber

[![Build Status](https://travis-ci.org/dalance/amber.svg?branch=master)](https://travis-ci.org/dalance/amber)
[![Circle CI](https://circleci.com/gh/dalance/amber.svg?style=svg)](https://circleci.com/gh/dalance/amber)
[![Build status](https://ci.appveyor.com/api/projects/status/o9n724jsag41gcre?svg=true)](https://ci.appveyor.com/project/dalance/amber)

**amber** is a code search and replace tool written by [Rust](https://www.rust-lang.org/). 
This tool is inspired by [ack](http://beyondgrep.com/), 
[ag](https://github.com/ggreer/the_silver_searcher), and other grep-like tools.

## Features

### Useful default settings
- Recursively search from the current directory
- Ignore vcs directories (.git, .hg, .svn, .bzr)
- Ignore binary files
- Output by the colored format

### Multi-threaded searching
Large files ( > 1MB by default) are divided and searched in parallel.

### Interactive replacing
**amber** can replace a keyword over directories (traditionally by `find ... | xargs sed -i '...'`) . 
You can decide to do replacing or not interactively.

## Install
Download from [release page](https://github.com/dalance/amber/releases/latest), and extract to the directory in PATH.

## Usage
Two commands (`ambs`/`ambr`) are provided. `ambs` means "amber search", and `ambr` means "amber replace".
The search keyword is not regular expression by default. If you want to use regular expression, add `--regex`.

```
ambs keyword                  // recursively search 'keyword' from the current directory. 
ambs keyword path             // recursively search 'keyword' from 'path'.
ambr keyword replacement      // recursively search 'keyword' from the current directory, and replace to 'replacement' interactively.
ambr keyword replacement path // recursively search 'keyword' from 'path', and replace to 'replacement' interactively.
```

**amber** replace interactively by default. If the keyword is found, the following prompt is shown, and wait.
If you input 'y', 'Y', 'Yes', the keyword is replaced. 'a', 'A', 'All' means replacing all keywords non-interactively.

```
Replace keyword? ( Yes[Y], No[N], All[A], Quit[Q] ):
```