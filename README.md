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

If `--regex` option is enabled, regex captures can be used in `replacemant` of `ambr`.

```
$ cat text.txt
aaa bbb
$ ambr --no-interactive --regex '(aaa) (?<pat>bbb)' '$1 $pat ${1} ${pat}' test.txt
$ cat text.txt
aaa bbb aaa bbb
```

## Benchmark

### Environment
- CPU: Xeon E5-2690 @ 2.90GHz
- MEM: 256GB
- OS : CentOS 7.2

### Data
- source1: https://github.com/torvalds/linux ( 52998files, 2.2GB )
- source2: https://dumps.wikimedia.org/jawiki/latest/jawiki-latest-pages-articles.xml.bz2 ( 1file, 8.5GB )

### Result

```
grep --color=auto -r EXPORT_SYMBOL_GPL ./data/linux  0.27s user 0.41s system  37% cpu 1.825 total
ag   --nogroup       EXPORT_SYMBOL_GPL ./data/linux  1.19s user 2.84s system 167% cpu 2.404 total
pt   --nogroup       EXPORT_SYMBOL_GPL ./data/linux  3.37s user 0.94s system 228% cpu 1.883 total
ambs                 EXPORT_SYMBOL_GPL ./data/linux  2.55s user 0.81s system 179% cpu 1.872 total
```

```
grep --color=auto -r "Quick Search" ./data/jawiki-latest-pages-articles.xml   0.82s user  1.68s system   99% cpu  2.495 total
ag   --nogroup       "Quick Search" ./data/jawiki-latest-pages-articles.xml  15.38s user  0.89s system  100% cpu 16.265 total
pt   --nogroup       "Quick Search" ./data/jawiki-latest-pages-articles.xml  12.49s user  1.13s system  100% cpu 13.548 total
ambs                 "Quick Search" ./data/jawiki-latest-pages-articles.xml   5.83s user 10.82s system 2304% cpu  0.723 total
```
