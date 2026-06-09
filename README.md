# anki-cli-rust

A command-line interface for managing [Anki](https://apps.ankiweb.net/) collections, designed to be
driven by both humans and coding agents. Instead of hand-rolled SQL against `collection.anki2`, every
operation goes through [Anki's official Rust backend](https://github.com/ankitects/anki/tree/main/rslib)
(`rslib`, pinned to the installed app version), so checksums, sync counters (`mod`/`usn`), protobuf
deck/notetype blobs, and card generation from templates are always handled exactly as the desktop app
would.

## Install

Requires a Rust toolchain and `protoc` (`brew install protobuf`).

```sh
cargo install --path .
# or just: cargo build --release  → target/release/anki-cli
```

## Usage

```sh
anki-cli decks list
anki-cli decks create "ML::Transformers"        # :: nests; parents auto-created
anki-cli decks rename "ML::Transformers" "ML::Attention"
anki-cli decks delete "ML::Attention"           # removes its cards too

anki-cli notetypes list
anki-cli notetypes show Cloze                   # fields + card templates

anki-cli notes add --deck ML --notetype Basic \
    -f "Front=What is the bias-variance tradeoff?" \
    -f "Back=Total error = bias² + variance + irreducible noise" \
    -t ml -t interview
anki-cli notes find "deck:ML tag:interview"     # full Anki search syntax
anki-cli notes show 1781041359199
anki-cli notes edit 1781041359199 -f "Back=..." --add-tag weak --remove-tag done
anki-cli notes delete 1781041359199

anki-cli cards list "is:suspended"
anki-cli cards move --deck "ML::Review" 1781041359202
anki-cli cards suspend 1781041359202
anki-cli cards unsuspend 1781041359202

anki-cli tags add "deck:ML is:due" priority     # bulk-tag by search
anki-cli tags remove "deck:ML" priority

anki-cli backup                                 # manual snapshot
```

### Agent-friendly output

Every command accepts a global `--json` flag that emits structured JSON on stdout (status messages go
to stderr). Note fields are keyed by field name in notetype order.

```sh
anki-cli --json notes find "tag:interview" | jq '.notes[].fields.Front'
```

### Collection discovery

The collection path is resolved in order:

1. `--collection /path/to/collection.anki2`
2. `$ANKI_COLLECTION`
3. Auto-detection of a single Anki profile under `~/Library/Application Support/Anki2` (macOS) or
   `~/.local/share/Anki2` (Linux). With multiple profiles, the CLI lists them and asks you to pick.

## Safety

- **Quit Anki before writing.** The desktop app holds an exclusive lock; the CLI detects this and
  refuses with a clear error rather than corrupting anything. (Reads also require the app closed.)
- **Automatic backups.** Before any mutating command, the collection file is copied to
  `anki-cli-backups/collection-<unix-ts>.anki2` next to the collection. Disable with `--no-backup`.
  To restore, quit Anki and copy a backup over `collection.anki2`.
- **No silent collection creation.** A mistyped path errors out instead of spawning an empty collection.
- **Sync-safe.** All mutations run through Anki's ops layer, so deletions land in the graveyard and
  modification counters are bumped — AnkiWeb sync sees CLI changes like any other edit.

## Version pinning

`Cargo.toml` pins the `anki` crate to the git tag matching the installed desktop app (currently
`25.09`). When you upgrade Anki, bump the tag and rebuild. The backend auto-upgrades older collection
schemas on open, just like the app itself — one more reason to keep the pin in lockstep.
