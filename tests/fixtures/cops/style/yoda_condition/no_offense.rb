x == 42

y != "hello"

1 == 1

obj == nil

flag == true

bar == :foo

done != false

value == CONST
count <= MAX_SIZE
total != Config::LIMIT

1 <= 3r
1 >= 1r
3 < 1r
1 > 1i
[[1, 2], [3, 4]] == [[1, 2], [3, 4]]
ConstPathRef <= { scope: (ConstPathRef | Const), name: Const }
%w(admin password) == [u, p]
[[1]] == [obj]
[[[1], 3], 2] == [[obj, 3], 2]
%i[UDP client] == [@transport, c.kind]

# Interpolated string (dstr) is literal in RuboCop — both sides constant
0 != "Local offset should not be zero for #{ENV['TZ']}"

# Interpolated string on LHS — RuboCop skips via interpolation? check
"#{interpolation}" == foo

# Interpolated regexp on LHS — also skipped by interpolation? check
/#{pattern}/ == text
