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

-> (foo, bar) { do_something }
         ^^^ Lint/UnusedBlockArgument: Unused block argument - `bar`.
    ^^^ Lint/UnusedBlockArgument: Unused block argument - `foo`.

->(arg) { 1 }
   ^^^ Lint/UnusedBlockArgument: Unused block argument - `arg`.

obj.method { |foo, *bars, baz| stuff(foo, baz) }
                    ^^^^ Lint/UnusedBlockArgument: Unused block argument - `bars`.

1.times do |index; block_local_variable|
                   ^^^^^^^^^^^^^^^^^^^^ Lint/UnusedBlockArgument: Unused block local variable - `block_local_variable`.
  puts index
end

define_method(:foo) do |bar|
                        ^^^ Lint/UnusedBlockArgument: Unused block argument - `bar`.
  puts :baz
end

-> (foo, bar) { puts bar }
    ^^^ Lint/UnusedBlockArgument: Unused block argument - `foo`.
