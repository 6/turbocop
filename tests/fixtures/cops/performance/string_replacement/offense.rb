str.gsub('a', 'b')
    ^^^^^^^^^^^^^^ Performance/StringReplacement: Use `tr` instead of `gsub` when replacing single characters.
str.gsub(' ', '-')
    ^^^^^^^^^^^^^^ Performance/StringReplacement: Use `tr` instead of `gsub` when replacing single characters.
str.gsub('x', 'y')
    ^^^^^^^^^^^^^^ Performance/StringReplacement: Use `tr` instead of `gsub` when replacing single characters.
