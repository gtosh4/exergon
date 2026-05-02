#!/usr/bin/env bash
# Print the next available voxel_id by finding the max used in assets/items/
max=$(grep -r 'voxel_id:' "$(dirname "$0")/../assets/items/" \
    | grep -o 'voxel_id: [0-9]*' \
    | awk '{print $2}' \
    | sort -n \
    | tail -1)
echo $((max + 1))
