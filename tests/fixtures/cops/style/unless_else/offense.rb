unless x
^^^^^^ Style/UnlessElse: Do not use `unless` with `else`. Rewrite these with the positive case first.
  a = 1
else
  a = 0
end

unless condition
^^^^^^ Style/UnlessElse: Do not use `unless` with `else`. Rewrite these with the positive case first.
  do_this
else
  do_that
end

unless foo?
^^^^^^ Style/UnlessElse: Do not use `unless` with `else`. Rewrite these with the positive case first.
  bar
else
  baz
end
