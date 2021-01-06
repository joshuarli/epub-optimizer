# epub-optimizer

**Do not use; heavy WIP.**

Update: I got too lazy to get this working, and this shell script I wrote works for me.

```bash
DIR="$(mktemp -d)" || exit

trap "rm -rf ${DIR}" EXIT

unzip "$1" -d "${DIR}/extracted"
cd "${DIR}/extracted"
fd -e jpg -e jpeg \
  -x jpegoptim -m85 -s {}
fd -e png \
  -x pngquant --skip-if-larger --quality=85 --strip --speed 1 --ext .png --force {}
# The mimetype file needs to be added first, and without compression.
zip -X0   "${DIR}/out.zip" mimetype
zip -X9ur "${DIR}/out.zip" .
# We could pass -u to update the source epub in-place, but meh.
cd -
mv "${DIR}/out.zip" "${1%.*}-optimized.epub"
```

Optimizes an epub file in-place.

`fd -e epub -x epub-optimizer {}`
