# rblint-expect: 1:0 Lint/RequireRangeParentheses: Wrap the endless range literal `1..` to avoid precedence ambiguity.
1..
42

# rblint-expect: 4:0 Lint/RequireRangeParentheses: Wrap the endless range literal `a..` to avoid precedence ambiguity.
a..
b

# rblint-expect: 7:0 Lint/RequireRangeParentheses: Wrap the endless range literal `x...` to avoid precedence ambiguity.
x...
y
