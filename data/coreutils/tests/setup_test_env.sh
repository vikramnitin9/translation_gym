#!/bin/sh
# This script sets up the environment to run the tests. These values were copied
# from the TESTS_ENVIRONMENT variable in the original coreutils Makefile.
# The environment variables have now been moved to the Dockerfiles

# Must be run from `coreutils` top level directory
# bash tests/setup_test_env.sh

. "tests/lang-default"

PATH='/usr/xpg4/bin:'"$PATH"
PATH='/usr/sbin:'"$PATH"

. "tests/envvar-check"

# Build the getlimits utility used by some tests
make --directory="getlimits/"
export PATH="getlimits:$PATH"

9>&2