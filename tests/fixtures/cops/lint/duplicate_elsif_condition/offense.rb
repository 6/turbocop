if x == 1
  :a
elsif x == 2
  :b
elsif x == 1
      ^^^^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  :c
end

if foo
  bar
elsif baz
  qux
elsif foo
      ^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  quux
end

if a > b
  1
elsif c > d
  2
elsif a > b
      ^^^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  3
end
