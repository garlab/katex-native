# Contributing

Issues and pull requests are welcome. katex-native applies KaTeX's CSS to katex-rs's HTML; parsing and the HTML itself come from katex-rs, so a formula that katex-rs rejects is a katex-rs issue.

## Development

Rust 1.88 or newer.

```sh
cargo fmt --check
cargo clippy --all-features --all-targets --locked -- -D warnings
cargo test --all-features --locked
cargo +1.88 check --all-features --all-targets --locked
```

`cargo test` compares the layouts of KaTeX's 128 screenshot cases with `tests/katex/layouts.txt`, so any change to layout shows up as a diff. When a change is intended, regenerate the snapshot with `UPDATE_LAYOUTS=1 cargo test --test katex_corpus`, review the diff, and measure the fidelity scores again (below).

The examples help when working on one formula:

```sh
cargo run --example items -- '\frac{a}{b}'                    # the display list
cargo run --example dump -- '\frac{a}{b}'                     # katex-rs's HTML
cargo run --features raster --example render -- '\frac{a}{b}' out.png
```

Keep everything in the repository suitable for publication: generic examples, relative paths, and no personal or machine-specific details. `src/engine.rs` translates KaTeX's stylesheet; keep its notice in `LICENSE` when copying more of it.

## Measuring fidelity

The README's scores compare renders with KaTeX's reference screenshots. With the cases from `ss_data.yaml` converted to JSON in `tests/katex/cases.json`:

```sh
cargo build --release --features raster --examples
target/release/examples/corpus tests/katex/cases.json /tmp/native
python3 -I tools/fetch_screenshots.py tests/katex/cases.json /tmp/katex-shots
python3 -I tools/compare.py tests/katex/cases.json /tmp/katex-shots /tmp/native /tmp/compare
```

`compare.py` needs Pillow and writes a comparison sheet per case and `scores.json`.

`tools/ss_data_to_json.rb` makes `tests/katex/cases.json`. To take the cases from another KaTeX commit, convert its `ss_data.yaml`, then regenerate `tests/katex/layouts.txt`, the layouts that `cargo test` compares against:

```sh
curl -sSfLo /tmp/ss_data.yaml https://raw.githubusercontent.com/KaTeX/KaTeX/<commit>/test/screenshotter/ss_data.yaml
ruby tools/ss_data_to_json.rb /tmp/ss_data.yaml tests/katex/cases.json
UPDATE_LAYOUTS=1 cargo test --test katex_corpus
```

Then replace `8000fbd4` wherever it appears (`git grep 8000fbd4`), including `fetch_screenshots.py`'s default commit, and measure the scores again.

## Releasing

The [release workflow](.github/workflows/release.yml) publishes version tags through crates.io's Trusted Publishing; the repository stores no registry token.

1. Set the new version in `Cargo.toml`, run `cargo check` so `Cargo.lock` follows, commit, and push `main`.
2. Tag that commit and push the tag:

   ```sh
   git tag -a vX.Y.Z -m release
   git push origin vX.Y.Z
   ```

The workflow checks that the tag matches `Cargo.toml` and is on `main`, runs the tests and a dry-run publish, then publishes. A version already on crates.io is skipped, so re-running is safe. Moving to a katex-rs release that changes the re-exported types is a breaking release.
