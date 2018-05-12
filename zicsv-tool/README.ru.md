[![crates.io](https://img.shields.io/crates/v/zicsv-tool.svg?maxAge=3600)](https://crates.io/crates/zicsv-tool)

[Same document in English](README.md)

# zicsv-tool

`zicsv-tool` - Утилита командной строки для разбора списков блокировок от
Zapret-Info в формате CSV.

## Установка

1. [Установите Rust](https://www.rust-lang.org/ru-RU/install.html).
2. Не забудьте обновить `PATH` в текущей сессии шелла:

    ```bash
    export PATH="${PATH}:${HOME}/.cargo/bin"
    ```

3. Скачайте, скомпилируйте и устновки `zicsv-tool`:

    ```bash
    cargo install zicsv-tool
    ```

## Использование

Скачайте свежий
[dump.csv](https://github.com/zapret-info/z-i/blob/master/dump.csv) перед тем
как делать что-либо ещё.

Поддерживаемые команды:

* `into-json` - Сконвертировать `dump.csv` в JSON.
* `search` - Поиск заблокированных адресов.
* `select` - Вывести выбранные типы заблокированных адресов.
* `updated` - Вывести дату последнего обновления `dump.csv`.

Обратите внимание, что по умолчанию утилита читает `dump.csv` из stdin и
пишет вывод в stdout.

### Помощь

```bash
zicsv-tool --help
zicsv-tool into-json --help
zicsv-tool search --help
zicsv-tool select --help
zicsv-tool updated --help
```

### Поиск записей по адресу

Пример:

```bash
$ zicsv-tool -i dump.csv search "http://google.com"
```

Пример вывода:

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

### Отладочные сообщения

Используется [pretty_env_logger](https://crates.io/crates/pretty_env_logger).
Чтобы увидеть отладночные сообщения, запустите следующую команду:

```bash
RUST_LOG=debug zicsv-tool [обычные опции]
```

Уровень журнала `trace` выключен в релизной сборке.
