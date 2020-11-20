# Elasticsearch Query CLI (eq)

A simple command line interface to perform queries on
[Elasticsearch](https://github.com/elastic/elasticsearch).

This uses the Official Elasticsearch Rust Client
[elasticsearch-rs](https://github.com/elastic/elasticsearch-rs), which is in an
*alpha* state.

# Usage

`eq` queries Elasticsearch for results and uses the [Search After][] API for
retrieving multiple batches of results. This can be useful for interacting with
documents like logs in a terminal.

[Search After]: https://www.elastic.co/guide/en/elasticsearch/reference/current/paginate-search-results.html#search-after

```
eq 1.0.0
A simple command line interface for Elasticsearch queries.

USAGE:
    eq [FLAGS] [OPTIONS]

FLAGS:
    -f, --follow                       Follow results, keep searching for new results until canceled
    -h, --help                         Prints help information
    -j, --json                         Print hits as newline delimited json objects, including all fields
    -n, --no-certificate-validation    Do not validate SSL/TLS certificate of server
    -V, --version                      Prints version information
    -v, --verbose                      Log extra information to stderr

OPTIONS:
    -a, --address <address>          The address of the Elasticsearch server to query [env: ES_ADDRESS=]  [default:
                                     http://localhost:9200]
    -b, --batch-size <batch-size>    The number of results to return per batch [default: 1000]
    -i, --index <index>              The index to query [default: filebeat-*]
    -l, --limit <limit>              The limit of results to return, 0 means no limit [default: 10000]
    -p, --password <password>        The Elasticsearch password to use [env: ES_PASSWORD]
    -q, --query <query>              The query string to search with [default: *]
    -Q, --query-dsl <query-dsl>      The query dsl json to search with, overrides --query [default: {}]
    -s, --sort <sort>                key:value pairs separated by commas to control sorting of results [default:
                                     @timestamp:asc,_id:asc]
    -u, --username <username>        The Elasticsearch username to use [env: ES_USERNAME=]
```

By default `_source.message` fields of results sorted by `@timestamp` are
logged to stdout.

```console
$ eq --index eq-testing
log entry 0
log entry 1
log entry 2
```

`--json` can be used to output search result hits as json objects and retrieve
all fields. A tool like [jq](https://stedolan.github.io/jq/) or
[gron](https://github.com/tomnomnom/gron) can be used to format or filter
fields for display as desired.

```console
$ eq --index eq-testing --follow --json | jq --raw-output '._source | "[\(."@timestamp")] \(.message)"'
[2020-03-10T18:11:38.988Z] log entry 0
[2020-03-11T18:11:38.988Z] log entry 1
[2020-03-12T18:11:38.988Z] log entry 2
^C

$ eq --index eq-testing --follow --json | gron --stream | grep '_source.message'
json[0]._source.message = "log entry 0";
json[1]._source.message = "log entry 1";
json[2]._source.message = "log entry 2";
^C
```

`--query` allows [Query string
syntax](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-query-string-query.html#query-string-syntax)
to be used.

```console
$ eq --query 'agent.hostname: my-server'
```

`--query-dsl` allows [Elasticsearch Query
DSL](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html)
to be used.

```console
$ eq --query-dsl '
{
  "query": {
    "term": {
      "agent.hostname": {
        "value": "my-server"
      }
    }
  }
}' > my-server.log
```

# Installation


## Binaries

[Precompiled binaries are available as github releases](https://github.com/Conky5/eq/releases).

### macOS Gatekeeper

The `eq` executable for macOS is not codesigned. If you download the
precompiled binary with a web browser an extended file attribute will be set on
the file that will cause a pop-up to appear when you try and run `eq`. To
prevent this I recommend removing all the extended attributes from the
`.tar.gz` after it has been downloaded to avoid Gatekeeper's interference:

```sh
xattr -c ~/Downloads/eq-v*.tar.gz
```

## Package managers

Homebrew/linuxbrew can be used with my [homebrew-tap](https://github.com/Conky5/homebrew-tap).

```sh
brew tap conky5/tap
brew install eq
```

## Cargo

If you have the rust tool chain installed, `eq` can be installed with
`cargo`:

```sh
cargo install --git https://github.com/Conky5/eq
```

# Development

Get [rustup](https://rustup.rs).

Run [cargo](https://doc.rust-lang.org/cargo/) commands for different tasks:

To build:

```sh
cargo build
```

To run like the executable will run:

```sh
cargo run
```

Create a release build:

```sh
cargo build --release
```

Auto format:

```sh
cargo fmt
```

Linting can be done with [clippy](https://github.com/rust-lang/rust-clippy)
which is installable via:

```sh
rustup component add clippy
```

Then run linting with:

```sh
cargo clippy
```

Generate documentation for **all** crates locally:

```sh
cargo doc
```

Then explore `./target/doc` to find documentation for crates.

# Testing

To run unit tests:

```sh
cargo test
```

To run tests with a real Elasticsearch instance, start one up accessible via
`http://localhost:9200` (for example by following [getting-started][] in the
Elasticsearch documentation) and run:

```sh
cargo test -- --ignored
```

An index named `eq-testing` will be used for testing.

[getting-started]: https://www.elastic.co/guide/en/elasticsearch/reference/current/getting-started-install.html#run-elasticsearch-local

# License

This is free software, licensed under [The Apache License Version 2.0.](LICENSE).
