unless x != y
^^^^^^ Style/InvertibleUnlessCondition: Use `if` with `==` instead of `unless` with `!=`.
  do_something
end
do_something unless x > 0
             ^^^^^^ Style/InvertibleUnlessCondition: Use `if` with `<=` instead of `unless` with `>`.
unless foo.even?
^^^^^^ Style/InvertibleUnlessCondition: Use `if` with `odd?` instead of `unless` with `even?`.
  bar
end
