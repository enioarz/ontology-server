#!/bin/bash
results=$(echo "$@" | xargs hyppo)
echo "results=$results" >> $GITHUB_OUTPUT
cp -R public /github/workspace/public
