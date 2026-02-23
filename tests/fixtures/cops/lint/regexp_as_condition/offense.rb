if /foo/
   ^^^^^ Lint/RegexpAsCondition: Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.
  do_something
end

if /bar/i
   ^^^^^^ Lint/RegexpAsCondition: Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.
  do_something
end

while /baz/
      ^^^^^ Lint/RegexpAsCondition: Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.
  do_something
end
