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

# Destructured block params: one element unused
translations.find { |(locale, translation)|
                              ^^^^^^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `translation`.
  locale.to_s == I18n.locale.to_s
}

# Destructured params in do..end block
hash.inject([]) do |array, (id, attributes)|
                            ^^ Lint/UnusedBlockArgument: Unused block argument - `id`.
  array << [attributes[:iso_code]]
end

# Lambda with destructured params
->((item_id, item_model)) {
    ^^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `item_id`.
  process(item_model: item_model)
}

# Lambda with mixed regular and destructured params
->(_, (item_id, item_model), _) {
       ^^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `item_id`.
  process(item_model: item_model)
}

# Destructured with splat inside: |(a, *b, c)|
items.each do |(first, *rest, last)|
                        ^^^^ Lint/UnusedBlockArgument: Unused block argument - `rest`.
  puts first
  puts last
end

# Unused block-pass parameter (&block)
obj.method do |original, env, &handler|
                               ^^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `handler`.
  original.call(env)
end

# Unused keyword rest parameter (**opts)
->(val, **opts) { val.to_s }
          ^^^^ Lint/UnusedBlockArgument: Unused block argument - `opts`.

# Unused keyword rest in block
do_something do |val, **options|
                        ^^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `options`.
  puts val
end

# Lambda as default parameter value in method def: unused `row`
def has_many(association_name, model:, foreign_key:, scope: ->(row) { true })
                                                               ^^^ Lint/UnusedBlockArgument: Unused block argument - `row`.
end

# Stabby lambda as default param with unused `b`
def one_opt_with_stabby(a = -> b { true }); end
                               ^ Lint/UnusedBlockArgument: Unused block argument - `b`.

# Lambda in keyword default with unused `node` in args param
def call_node?(node, name:, args: ->(node) { true })
                                     ^^^^ Lint/UnusedBlockArgument: Unused block argument - `node`.
end

# Bare `super` inside a block should not mark block args as used —
# super forwards enclosing method args, not block params
def transaction(*args)
  @connection.execute(false) do |conn|
                                 ^^^^ Lint/UnusedBlockArgument: Unused block argument - `conn`.
    super
  end
end

# define_method block with super: block arg is unused
class Renderer
  class << self
    %w{renderables}.each do |m|
      define_method(m) do |*args|
                            ^^^^ Lint/UnusedBlockArgument: Unused block argument - `args`.
        super
      end
    end
  end
end

# Another method with super inside block
def log_yield(sql, conn)
  instrument.span("SQL") do |span|
                             ^^^^ Lint/UnusedBlockArgument: Unused block argument - `span`.
    super
  end
end

# define_method with non-splat param and super
class Messageable
  define_method(:email) do |object|
                            ^^^^^^ Lint/UnusedBlockArgument: Unused block argument - `object`.
    super
  end
end
