# Runs one of a task's build steps, with caches for the compilers it might use.
#
# $1 is the step's command. It's an argument and not pasted in here so it never needs quoting.
#
# The caches only make things faster, so whatever can't be set up is left out and the step runs without it. They're
# split by user because one user can't write to another's.

command=$1
user=$(id -u 2>/dev/null || echo unknown)
shared=/gofer-cache/shared/$user

# From here on the arguments are the links to remove at the end.
set --

if mkdir -p "$shared/cargo/registry" "$shared/cargo/git" 2>/dev/null; then
    # Go compares what's in the files, so it can always use these.
    export GOCACHE="$shared/go-build" GOMODCACHE="$shared/go-mod"

    # Cargo has no setting for where it keeps its downloads, short of moving all of CARGO_HOME, which would lose the
    # image's own Cargo config and change where `cargo install` puts things. So link the parts we want, if the image
    # doesn't have them already.
    #
    # The first two are the locks that stop two builds from unpacking the same crate at once. The downloads can't be
    # shared unless those are too, hence the break.
    cargo_home=${CARGO_HOME:-$HOME/.cargo}

    if [ -d "$cargo_home" ]; then
        for name in .package-cache .package-cache-mutate registry git; do
            [ ! -e "$cargo_home/$name" ] || break
            ln -s "$shared/cargo/$name" "$cargo_home/$name" 2>/dev/null || break
            set -- "$@" "$cargo_home/$name"
        done
    fi
fi

# This keeps what Cargo compiles in a cache. The finished program isn't part of that, so it still ends up in the image.
# A cache over target/ would be simpler, but then the program would be in the cache and missing from the image.
#
# Cargo only rebuilds a file that's newer than the last build, and the files that were copied in can be old. Say version
# A of a file gets built, then version B, and then the user goes back to A. The copy of A comes from Docker's cache with
# its old time, so to Cargo it looks like nothing changed since B, and we'd ship B. Making every copied file new avoids
# that. If we can't, which is usually because we're not root, then using the cache isn't safe.
#
# The list can name files that don't exist. A step might have removed one, and for some copies it has two guesses.
#
# https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir
if [ -f /gofer-run/files ] && xargs -0 -n 500 sh -c '
    count=$#
    for file; do [ ! -f "$file" ] || set -- "$@" "$file"; done
    shift "$count"
    [ $# -eq 0 ] || touch "$@"
' sh </gofer-run/files 2>/dev/null; then
    export CARGO_BUILD_BUILD_DIR="/gofer-cache/rust/$user"

    # Older Cargo ignores that setting, so everything gets compiled every time. Gofer reads this line and suggests a
    # newer image.
    version=$(cargo -V 2>/dev/null | sed -n 's/^cargo 1\.\([0-9]*\)\..*/\1/p')
    if [ -n "$version" ] && [ "$version" -lt 91 ]; then
        echo "gofer: Cargo 1.$version can't keep what it compiles between builds; Cargo 1.91 and newer can" >&2
    fi
fi

/bin/sh -c "$command"
code=$?

# None of this belongs in the image.
[ $# -eq 0 ] || rm -f "$@"

exit "$code"
