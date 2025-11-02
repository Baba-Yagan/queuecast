#!/bin/bash

# script to open thunar with the original file location
# works with hardlinks and softlinks
# useful for when opening files to have correct filepath in mpv for sub loading etc 

if [ $# -eq 0 ]; then
    echo "usage: $0 <file_path>"
    echo "opens thunar with the original location of a symlink or hardlink"
    exit 1
fi

file_path="$1"

if [ ! -e "$file_path" ]; then
    echo "error: file '$file_path' does not exist"
    exit 1
fi

# check if it's a symlink
if [ -L "$file_path" ]; then
    # resolve symlink to get the original file
    original_path=$(readlink -f "$file_path")
    echo "symlink detected. opening and selecting original file: $original_path"
    thunar "$original_path" &
elif [ -f "$file_path" ]; then
    # for regular files and hardlinks, check if there are other links
    inode=$(stat -c %i "$file_path")
    link_count=$(stat -c %h "$file_path")
    
    if [ "$link_count" -gt 1 ]; then
        echo "hardlink detected (inode: $inode, links: $link_count)"
        # for hardlinks, we'll select the given file since hardlinks don't have a concept of "original"
        resolved_path=$(realpath "$file_path")
        echo "opening and selecting hardlink file: $resolved_path"
        thunar "$resolved_path" &
    else
        # regular file, select it in thunar
        resolved_path=$(realpath "$file_path")
        echo "regular file. opening and selecting: $resolved_path"
        thunar "$resolved_path" &
    fi
else
    echo "error: '$file_path' is not a regular file or symlink"
    exit 1
fi
