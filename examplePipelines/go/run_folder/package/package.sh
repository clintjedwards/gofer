set -eu

echo "packaging as $(id -un)"
version=$(cat /gofer/run/version)

# The build task ran as root, so this task can read its files but not change them. Anyone can make new folders in the
# run folder itself, though.
mkdir -p /gofer/run/release

for dir in /gofer/run/dist/*/; do
	platform=$(basename "$dir")
	tar -czf "/gofer/run/release/simple-$version-$platform.tar.gz" -C "$dir" simple
done

cd /gofer/run/release
sha256sum -- *.tar.gz > SHA256SUMS
cat SHA256SUMS
