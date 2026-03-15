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

if x == Module::CONSTANT1
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == Module::CONSTANT2
elsif x == Another::CONST3
else
end

if (x == 1)
^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif (x == 2)
elsif (x == 3)
end

if (1..10).include?(x)
^^^^^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif (11...100).include?(x)
elsif (200..300).include?(x)
end

if /foo/ =~ x
^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x =~ /bar/
elsif /baz/ =~ x
end

if /foo/.match?(x)
^^^^^^^^^^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x.match?(/bar/)
elsif x.match?(/baz/)
end

# Long chain should produce only ONE offense (at the top-level if)
if x == 1
^^^^^^^^^ Style/CaseLikeIf: Convert `if-elsif` to `case-when`.
elsif x == 2
elsif x == 3
elsif x == 4
elsif x == 5
end
