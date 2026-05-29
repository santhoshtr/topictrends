#!/bin/bash
# Fetch and decompress MediaWiki history dumps for a wiki, streaming to stdout.
#
# Usage:
#   fetch_pageedits.sh --check WIKI SNAPSHOT       Probe for dump availability.
#                                                  Exit 0 if present, non-zero on 404.
#   fetch_pageedits.sh WIKI SNAPSHOT MIN_EDIT_YEAR Stream concatenated, decompressed
#                                                  dump content to stdout. Logs to stderr.
#
# Designed to be piped into get-pageedits:
#   fetch_pageedits.sh enwiki 2026-04 2025 | get-pageedits enwiki out.parquet

set -euo pipefail

if [ "${1:-}" = "--check" ]; then
    WIKI="${2:?usage: $0 --check WIKI SNAPSHOT}"
    SNAPSHOT="${3:?usage: $0 --check WIKI SNAPSHOT}"
    BASE_URL="https://dumps.wikimedia.org/other/mediawiki_history/${SNAPSHOT}/${WIKI}/"
    curl -fsSI -o /dev/null "$BASE_URL"
    exit $?
fi

WIKI="${1:?usage: $0 WIKI SNAPSHOT MIN_EDIT_YEAR}"
SNAPSHOT="${2:?usage: $0 WIKI SNAPSHOT MIN_EDIT_YEAR}"
MIN_EDIT_YEAR="${3:?usage: $0 WIKI SNAPSHOT MIN_EDIT_YEAR}"

BASE_URL="https://dumps.wikimedia.org/other/mediawiki_history/${SNAPSHOT}/${WIKI}/"
TEMP_DIR="$(mktemp -d -t "topictrend-${WIKI}-pageedits.XXXXXX")"
trap 'rm -rf "$TEMP_DIR"' EXIT

echo "Processing page edits for $WIKI from snapshot $SNAPSHOT..." >&2
echo "Fetching file list from $BASE_URL" >&2

FILELIST="$TEMP_DIR/filelist.txt"
if ! wget -q -O - "$BASE_URL" | grep -oP 'href="\K[^"]*\.bz2(?=")' > "$FILELIST"; then
    echo "Error fetching file list from $BASE_URL" >&2
    exit 1
fi

if [ ! -s "$FILELIST" ]; then
    echo "No .bz2 files found at $BASE_URL" >&2
    exit 1
fi

echo "Filtering files by year (MIN_EDIT_YEAR=$MIN_EDIT_YEAR)..." >&2
FILTERED_LIST="$TEMP_DIR/filtered.txt"
SKIPPED=0
while IFS= read -r filename; do
    year_part="$(echo "$filename" | cut -d'.' -f3)"
    if [ "$year_part" = "all-time" ]; then
        echo "$filename" >> "$FILTERED_LIST"
    else
        year="${year_part:0:4}"
        if [[ "$year" =~ ^[0-9]{4}$ ]] && [ "$year" -ge "$MIN_EDIT_YEAR" ]; then
            echo "$filename" >> "$FILTERED_LIST"
        else
            SKIPPED=$((SKIPPED+1))
        fi
    fi
done < "$FILELIST"

if [ ! -s "$FILTERED_LIST" ]; then
    echo "No files remain after year filtering (MIN_EDIT_YEAR=$MIN_EDIT_YEAR)" >&2
    exit 1
fi

if [ "$SKIPPED" -gt 0 ]; then
    echo "Skipped $SKIPPED file(s) before year $MIN_EDIT_YEAR" >&2
fi

TOTAL=$(wc -l < "$FILTERED_LIST")
echo "Found $TOTAL dump file(s) to download and process" >&2

i=0
while IFS= read -r filename; do
    i=$((i+1))
    echo "[$i/$TOTAL] Downloading $filename..." >&2
    FILE_PATH="$TEMP_DIR/$filename"
    if ! wget -q --show-progress -O "$FILE_PATH" "${BASE_URL}${filename}"; then
        echo "Error downloading $filename" >&2
        exit 1
    fi
    echo "[$i/$TOTAL] Decompressing $filename..." >&2
    if ! bzip2 -dc "$FILE_PATH"; then
        echo "Error decompressing $filename" >&2
        exit 1
    fi
    rm -f "$FILE_PATH"
    echo "[$i/$TOTAL] Deleted $filename to free disk space" >&2
done < "$FILTERED_LIST"
