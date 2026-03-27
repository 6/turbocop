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

# Block literal does not make the redundant argument non-redundant
arr.sum(0) { |x| x * 2 }
       ^^^ Style/RedundantArgument: Argument `0` is redundant because it is implied by default.

result = ary.each.sum(0) {|x| yielded << x; 2*x }
                     ^^^ Style/RedundantArgument: Argument `0` is redundant because it is implied by default.

returned_object = "chunky bacon".split(" ") { |str| a << str.capitalize }
                                      ^^^^^ Style/RedundantArgument: Argument `" "` is redundant because it is implied by default.

# do..end block also should not suppress detection
str.split(" ") do |s|
         ^^^^^ Style/RedundantArgument: Argument `" "` is redundant because it is implied by default.
  puts s
end
