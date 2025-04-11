#!/bin/sh
# Verify that tail works even when it's reading from a file
# that is not at its beginning.  Based on a report from John Roll.

# Copyright (C) 2001-2024 Free Software Foundation, Inc.

# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

SCRIPTPATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
. "$SCRIPTPATH/../../tests/init.sh"; path_prepend_ $1

(echo 1; echo 2) > k || framework_failure_


sh -c 'read x; tail' < k > out || fail=1
cat <<EOF > exp
2
EOF

compare exp out || fail=1

Exit $fail
