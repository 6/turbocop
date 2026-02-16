# rblint-expect: 1:0 Lint/UselessElseWithoutRescue: `else` without `rescue` is useless.
# This syntax is invalid in Ruby 2.6+
# The parser will reject it before the cop runs
