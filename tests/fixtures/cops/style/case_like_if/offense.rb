if x == 1
^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == 2
elsif x == 3
else
end

if Integer === x
^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif /foo/ === x
elsif (1..10) === x
else
end

if x == CONSTANT1
^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif CONSTANT2 == x
elsif CONSTANT3 == x
else
end
