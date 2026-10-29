#!/bin/sh
# Launch the built viewer with prompt release of large malloc allocations.
set -eu

if [ "$(uname -s)" != Darwin ]; then
    echo "This launcher is for macOS. Run target/release/panel-y directly on other systems." >&2
    exit 1
fi

viewer_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
viewer_binary="$viewer_dir/target/release/panel-y"
if [ ! -x "$viewer_binary" ]; then
    echo "Build first: cargo build --release --locked --manifest-path $viewer_dir/Cargo.toml" >&2
    exit 1
fi

# libmalloc reads this at process startup. Honor an explicit caller override;
# setting it inside the Rust process is too late to configure the allocator.
exec env MallocLargeCache="${MallocLargeCache:-0}" "$viewer_binary" "$@"
