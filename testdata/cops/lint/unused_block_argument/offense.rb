do_something do |used, unused|
                       ^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `unused`.
  puts used
end

do_something do |bar|
                 ^^^ Lint/UnusedBlockArgument: Unused block argument - `bar`.
  puts :foo
end

[1, 2, 3].each do |x|
                   ^ Lint/UnusedBlockArgument: Unused block argument - `x`.
  puts "hello"
end
