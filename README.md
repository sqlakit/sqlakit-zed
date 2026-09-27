# sqlakit-zed

A [Zed](https://zed.dev) extension for [SQLAKit](https://sqlakit.readthedocs.io/en/stable/)
templates. It runs [`sqlakit-lsp`](https://github.com/sqlakit/sqlakit-lsp),
which shows errors as you type, completes macro names, shows the SQL a macro
writes on hover, and finds definitions and references. It needs `sqlakit` 0.21
or newer.

## Install

Install the SQL extension first: **zed: extensions**, search for SQL.

Until this extension is published, install it from this directory: open the
command palette, run **zed: install dev extension**, and pick the directory.
Zed builds it with `rustup`, which needs Rust 1.85 or newer.

If [uv](https://docs.astral.sh/uv/) is installed, nothing else is needed: the
extension runs the server with `uvx`. You can also add the server to the
project:

```console
$ uv add --dev sqlakit-lsp
```

## Where the server comes from

The extension uses the first one it finds:

1. `lsp.sqlakit-lsp.binary.path` in Zed's settings
2. `.venv/bin/sqlakit-lsp` in the project
3. `sqlakit-lsp` on the `PATH`
4. `uvx sqlakit-lsp`, a 0.2 release, with the `sqlakit` version from the
   project's `uv.lock`

The server starts only in projects that list `sqlakit` in `pyproject.toml`,
`uv.lock` or `requirements.txt`. Set the path in the settings to start it
anywhere else:

```json
{
  "lsp": {
    "sqlakit-lsp": {
      "binary": { "path": "/path/to/env/bin/sqlakit-lsp" }
    }
  }
}
```

To color macro calls and parameters, add `"semantic_tokens": "combined"` to
the settings.

**dev: open language server logs** shows what the server found in the
project: template directories, macros and the database dialect.

If the project sets `languages.Python.language_servers`, add `"sqlakit-lsp"`
to that list, or keep `"..."` in it.

## Try it

[`sqlakit-example`](https://github.com/sqlakit/sqlakit-example) is a small
project to try it on. Run `uv sync`, open it in Zed, open
`app/sql/users/search.sql` and type `tpl.`.

## Development

```console
$ rustup target add wasm32-wasip2
$ cargo test
$ cargo build --release --target wasm32-wasip2
```
