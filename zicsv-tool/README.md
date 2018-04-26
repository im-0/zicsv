[![crates.io](https://img.shields.io/crates/v/zicsv-tool.svg?maxAge=3600)](https://crates.io/crates/zicsv-tool)

[Этот же документ на русском](README.ru.md)

# zicsv-tool

`zicsv-tool` - Command-line tool for parsing Zapret-Info CSV lists.

## Installation

1. [Install Rust](https://www.rust-lang.org/en-US/install.html).
2. Do not forget to update `PATH` in current shell session:

    ```bash
    export PATH="${PATH}:${HOME}/.cargo/bin"
    ```

3. Download, compile and install `zicsv-tool`:

    ```bash
    cargo install zicsv-tool
    ```

## Usage

Download fresh
[dump.csv](https://github.com/zapret-info/z-i/blob/master/dump.csv) before
doing everything else.

Supported commands:

* `into-json` - Convert `dump.csv` into JSON format.
* `search` - Search blocked addresses.
* `select` - Print selected types of blocked addresses.
* `updated` - Print date of last update of `dump.csv`.

Note that by default this tool reads `dump.csv` from stdin and writes any
output to stdout.

### Help

```bash
zicsv-tool --help
zicsv-tool into-json --help
zicsv-tool search --help
zicsv-tool select --help
zicsv-tool updated --help
```

### Searching records by address

Example:

```bash
$ zicsv-tool -i dump.csv search "http://google.com"
```

Example output:

```
http://google.com:
    http://google.com/: not found

    google.com: not found

    74.125.205.100: blocked
        IPv4 address is equal to blocked IPv4 address:
            Blocked: 74.125.205.100
            Organization: Генпрокуратура
            Document ID: 27-31-2018/Ид2971-18
            Document date: 2018-04-16

    74.125.205.138: not found

    74.125.205.102: blocked
        IPv4 address is equal to blocked IPv4 address:
            Blocked: 74.125.205.102
            Organization: Генпрокуратура
            Document ID: 27-31-2018/Ид2971-18
            Document date: 2018-04-16

    74.125.205.113: blocked
        IPv4 address is equal to blocked IPv4 address:
            Blocked: 74.125.205.113
            Organization: Генпрокуратура
            Document ID: 27-31-2018/Ид2971-18
            Document date: 2018-04-16

    74.125.205.139: not found

    74.125.205.101: blocked
        IPv4 address is equal to blocked IPv4 address:
            Blocked: 74.125.205.101
            Organization: Генпрокуратура
            Document ID: 27-31-2018/Ид2971-18
            Document date: 2018-04-16
```
