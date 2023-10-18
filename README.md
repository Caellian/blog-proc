# Blog Manager

Flat FS blog content manager and markdown processor.

The idea behind this thing is that you just point it at a git repository or a directory containing Markdown files and it generates HTML files and JSON index files ready for static consumption.

Git integration provides edit history for posts.

Sources can have a `yaml` header which starts and ends with a line containing only `---`.

```
cargo run -- -w ./posts -e svelte build
```

## Attribution

- Code for rendering `LaTeX` blocks is roughly based on [`just-latex`](https://github.com/ma-chengyuan/just-latex/tree/main) (not yet implemented)
  - `just-latex` is a pandoc filter while this is intended to be a self
    contained binary.

## License

This project is licensed under [zlib](./LICENSE_ZLIB), [MIT](./LICENSE_MIT), or [Apache-2.0](./LICENSE_APACHE) license, choose whichever suits you most.
