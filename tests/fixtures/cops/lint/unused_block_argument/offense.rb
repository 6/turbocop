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

# Variable shadowing in nested blocks: outer `item` is unused because
# inner block shadows it — the read of `item` inside refers to the inner param
items.each do |item|
               ^^^^ Lint/UnusedBlockArgument: Unused block argument - `item`.
  results.map do |item|
    item.name
  end
end

# Nested lambda shadows outer param
records.each do |record|
                 ^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `record`.
  -> (record) { record.save }
end

# Multiple levels of nesting with shadowing
data.each do |value|
              ^^^^^ Lint/UnusedBlockArgument: Unused block argument - `value`.
  items.each do |value|
    value.process
  end
end

# Blocks inside def methods should still detect unused args
def process_items
  items.each do |item|
                 ^^^^ Lint/UnusedBlockArgument: Unused block argument - `item`.
    puts "processing"
  end
end

# Block inside class > def
class Worker
  def run
    tasks.each do |task|
                   ^^^^ Lint/UnusedBlockArgument: Unused block argument - `task`.
      puts "running"
    end
  end
end

# Nested module > class > def > block
module Services
  class Processor
    def call
      records.map do |record|
                      ^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `record`.
        "done"
      end
    end
  end
end
