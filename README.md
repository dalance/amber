# amber

[![Actions Status](https://github.com/dalance/amber/workflows/Regression/badge.svg)](https://github.com/dalance/amber/actions)
[![Crates.io](https://img.shields.io/crates/v/amber.svg)](https://crates.io/crates/amber)
[![codecov](https://codecov.io/gh/dalance/amber/branch/master/graph/badge.svg)](https://codecov.io/gh/dalance/amber)

**amber** is a code search and replace tool written by [Rust](https://www.rust-lang.org/).
This tool is inspired by [ack](http://beyondgrep.com/),
[ag](https://github.com/ggreer/the_silver_searcher), and other grep-like tools.

## Features

### Useful default settings
- Recursively search from the current directory
- Ignore VCS directories (.git, .hg, .svn, .bzr)
- Ignore binary files
- Output by the colored format

### Multi-threaded searching
Large files ( > 1MB by default) are divided and searched in parallel.

### Interactive replacing
**amber** can replace a keyword over directories (traditionally by `find ... | xargs sed -i '...'`) .
You can decide to do replacing or not interactively.

## Installation

### Arch Linux
Install the `amber-search-git` package from AUR.

```
yaourt -S amber-search-git
```

### Cargo

You can install with [cargo](https://crates.io/crates/amber).

```
cargo install amber
```

### Manual
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

If `--regex` option is enabled, regex captures can be used in `replacement` of `ambr`.

```
$ cat text.txt
aaa bbb
$ ambr --no-interactive --regex '(aaa) (?<pat>bbb)' '$1 $pat ${1} ${pat}' test.txt
$ cat text.txt
aaa bbb aaa bbb
```

## Configuration

### Configuration path

You can change configuration by writing a configuration file.
The locations of the configuration file is OS-specific:

 * Linux: `~/.config/amber/ambs.toml`, `/etc/amber/ambs.toml`
 * macOS: `~/Library/Preferences/com.github.dalance.amber/ambs.toml`, `/etc/amber/ambs.toml`
 * Windows: `~/AppData/Roaming/dalance/amber/config/ambs.toml`

For compatibility, if `~/.ambs.toml` exists, it will be preferred to
the OS-specific locations.

The above paths are examples for the configuration of `ambs` command.
`ambr.toml` in the same directory is used for `ambr` command.

### Configurable value

Available entries and default values are below:

```toml
regex          = false
column         = false
row            = false
binary         = false
statistics     = false
skipped        = false
interactive    = true
recursive      = true
symlink        = true
color          = true
file           = true
skip_vcs       = true
skip_gitignore = true
fixed_order    = true
parent_ignore  = true
line_by_match  = false
```

You can choose some entries to override like below:

```toml
column = true
```

## Benchmark

### Environment

- CPU: Intel(R) Xeon(R) Gold 6134 CPU @ 3.20GHz
- MEM: 1.5TB
- OS : CentOS 7.5

### Target Data

- source1: https://github.com/torvalds/linux ( 52998files, 2.2GB )
- source2: https://dumps.wikimedia.org/jawiki/latest/jawiki-latest-pages-articles.xml.bz2 ( 1file, 8.5GB )

### Pattern

- pattern1( many files with many matches ) : 'EXPORT_SYMBOL_GPL' in source1
- pattern2( many files with few matches  ) : 'irq_bypass_register_producer' in source1
- pattern3( a large file with many matches ) : '検索結果' in source2
- pattern4( a large file with few matches  ) : '"Quick Search"' in source2

### Comparison Tools

- amber (v0.5.1)
- [ripgrep](https://github.com/BurntSushi/ripgrep) (v0.10.0)
- [grep](https://www.gnu.org/software/grep/) (v2.20)
- [fastmod](https://github.com/facebookincubator/fastmod) (v0.2.0)
- [find](https://www.gnu.org/software/findutils/)/[sed](https://www.gnu.org/software/sed/) (v4.5.11/v4.2.2)

### Benchmarking Tool

[hyperfine](https://github.com/sharkdp/hyperfine) with the following options.

- `--warmup 3`: to load all data on memory.

### Result

- search ( `compare_ambs.sh` )

| pattern | amber            | ripgrep          | grep             |
| ------- | ---------------- | ---------------- | ---------------- |
| 1       | 212.8ms ( 139% ) | 154.1ms ( 100% ) | 685.2ms ( 448% ) |
| 2       | 199.7ms ( 132% ) | 151.6ms ( 100% ) | 678.7ms ( 448% ) |
| 3       | 1.068s  ( 100% ) | 4.642s  ( 434% ) | 3.869s  ( 362% ) |
| 4       | 1.027s  ( 100% ) | 4.409s  ( 429% ) | 3.118s  ( 304% ) |

- replace ( `compare_ambr.sh` )

| pattern | amber            | fastmod          | find/sed            |
| ------- | ---------------- | ---------------- | ------------------- |
| 1       | 792.2ms ( 100% ) | 1231ms  ( 155% ) | 155724ms ( 19657% ) |
| 2       | 418.1ms ( 119% ) | 352.4ms ( 100% ) | 157396ms ( 44663% ) |
| 3       | 18.390s ( 100% ) | 74.282s ( 404% ) | 639.740s ( 3479% )  |
| 4       | 17.777s ( 100% ) | 74.204s ( 417% ) | 625.756s ( 3520% )  |
