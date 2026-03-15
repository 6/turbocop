array.join('')
          ^^^^ Style/RedundantArgument: Argument `''` is redundant because it is implied by default.

exit(true)
    ^^^^^^ Style/RedundantArgument: Argument `true` is redundant because it is implied by default.

exit!(false)
     ^^^^^^^ Style/RedundantArgument: Argument `false` is redundant because it is implied by default.

# Multiline receiver - offense at argument parens, not receiver start
[
  1,
  2,
  3
].join("")
      ^^^^ Style/RedundantArgument: Argument `""` is redundant because it is implied by default.

# Chained multiline call
items
  .map(&:to_s)
  .join('')
       ^^^^ Style/RedundantArgument: Argument `''` is redundant because it is implied by default.

str.split(" ")
         ^^^^^ Style/RedundantArgument: Argument `" "` is redundant because it is implied by default.

str.chomp("\n")
         ^^^^^^ Style/RedundantArgument: Argument `"\n"` is redundant because it is implied by default.

str.to_i(10)
        ^^^^ Style/RedundantArgument: Argument `10` is redundant because it is implied by default.

arr.sum(0)
       ^^^ Style/RedundantArgument: Argument `0` is redundant because it is implied by default.
