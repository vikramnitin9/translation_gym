#!/bin/sh
# show that 'split --lines=2' works.

# Copyright (C) 2002-2024 Free Software Foundation, Inc.

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

printf '1\n2\n3\n4\n5\n' > in || framework_failure_

split --lines=2 in > out || fail=1
cat <<\EOF > exp-1
1
2
EOF
cat <<\EOF > exp-2
3
4
EOF
cat <<\EOF > exp-3
5
EOF

compare exp-1 xaa || fail=1
compare exp-2 xab || fail=1
compare exp-3 xac || fail=1
test -f xad && fail=1

Exit $fail
