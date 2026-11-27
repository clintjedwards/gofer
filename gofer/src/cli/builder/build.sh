# Runs one build inside the BuildKit container. The image comes out on stdout and BuildKit's progress on stderr.
#
# $1 is the folder that the build's files were copied into, laid out the way dockerfile.rs describes. $2 is either
# empty or --no-cache.
#
# The CLI keeps stdin open for as long as it wants the build, and nothing is ever written to it. Stopping the CLI's
# side of an exec doesn't stop the exec, so if the CLI is stopped any other way, by Ctrl-C, a crash or even kill -9, the
# build would carry on without anyone waiting for it. What does happen every time is that stdin closes, so we wait for
# that and stop the build ourselves.

dir=$1
no_cache=$2

# A task that copies nothing in has nothing to put in its context, but buildctl won't start without the folder.
mkdir -p "$dir/context"

exec 3<&0
# shellcheck disable=SC2086 # $no_cache is either nothing or one flag.
buildctl build --frontend dockerfile.v0 --progress=rawjson $no_cache \
    --local context="$dir/context" --local dockerfile="$dir" \
    --opt platform=linux/amd64 --output type=docker,dest=- </dev/null &
build=$!

(cat <&3 >/dev/null; kill -INT "$build" 2>/dev/null) >/dev/null 2>&1 &
watcher=$!

wait "$build"
code=$?
kill "$watcher" 2>/dev/null
rm -rf "$dir"
exit "$code"
