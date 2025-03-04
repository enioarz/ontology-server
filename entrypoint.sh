#!/bin/sh -l

results=$(echo "$@" | xargs hyppo)
echo "results=$results" >> $GITHUB_OUTPUT
