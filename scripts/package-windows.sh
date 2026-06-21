#!/usr/bin/env bash
set -euo pipefail

prefix="${MINGW_PREFIX:-/ucrt64}"
dist="${1:-dist}"

rm -rf "$dist"
mkdir -p "$dist/bin" "$dist/lib" "$dist/share" "$dist/etc"

cp target/release/42host.exe "$dist/bin/"
cp LICENSE README.md "$dist/"

copy_tree() {
    local source="$1"
    local destination="$2"
    if [[ -e "$source" ]]; then
        mkdir -p "$destination"
        cp -a "$source" "$destination/"
    fi
}

# GTK loads these modules and data files dynamically, so ldd alone cannot find them.
copy_tree "$prefix/lib/gdk-pixbuf-2.0" "$dist/lib"
copy_tree "$prefix/lib/gio" "$dist/lib"
copy_tree "$prefix/lib/gtk-4.0" "$dist/lib"
copy_tree "$prefix/share/glib-2.0" "$dist/share"
copy_tree "$prefix/share/icons" "$dist/share"
copy_tree "$prefix/share/themes" "$dist/share"
copy_tree "$prefix/share/mime" "$dist/share"
copy_tree "$prefix/share/fontconfig" "$dist/share"
copy_tree "$prefix/etc/fonts" "$dist/etc"

for tool in gdk-pixbuf-query-loaders.exe gio-querymodules.exe gtk4-update-icon-cache.exe; do
    if [[ -f "$prefix/bin/$tool" ]]; then
        cp "$prefix/bin/$tool" "$dist/bin/"
    fi
done

# Copy every transitive MinGW DLL dependency of the app, helper tools and GTK modules.
mapfile -d '' queue < <(find "$dist" -type f \( -iname '*.exe' -o -iname '*.dll' \) -print0)
declare -A scanned=()

while ((${#queue[@]})); do
    file="${queue[0]}"
    queue=("${queue[@]:1}")

    key="$(cygpath -aw "$file")"
    if [[ -n "${scanned[$key]:-}" ]]; then
        continue
    fi
    scanned[$key]=1

    while IFS= read -r dependency; do
        [[ -f "$dependency" ]] || continue
        destination="$dist/bin/$(basename "$dependency")"
        if [[ ! -f "$destination" ]]; then
            cp "$dependency" "$destination"
            queue+=("$destination")
        fi
    done < <(ldd "$file" 2>/dev/null | awk -v bin="$prefix/bin/" '$3 ~ "^" bin { print $3 }' | sort -u)
done

echo "Packaged $(find "$dist" -type f | wc -l) files in $dist"
