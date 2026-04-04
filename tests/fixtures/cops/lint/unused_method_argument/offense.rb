def some_method(used, unused)
                      ^^^^^^ Lint/UnusedMethodArgument: Unused method argument - `unused`.
  puts used
end

def foo(bar, baz)
             ^^^ Lint/UnusedMethodArgument: Unused method argument - `baz`.
  bar
end

def calculate(x, y, z)
                    ^ Lint/UnusedMethodArgument: Unused method argument - `z`.
  x + y
end

def protect(*args)
             ^^^^ Lint/UnusedMethodArgument: Unused method argument - `args`.
  do_something
end

# block parameter unused
def with_block(a, &block)
                   ^^^^^ Lint/UnusedMethodArgument: Unused method argument - `block`.
  puts a
end

# keyword rest parameter unused
def with_kwrest(a, **opts)
                     ^^^^ Lint/UnusedMethodArgument: Unused method argument - `opts`.
  puts a
end

# post parameter unused (after rest)
def with_post(*args, last)
                     ^^^^ Lint/UnusedMethodArgument: Unused method argument - `last`.
  args.first
end

# multi-assign target only (not a read)
def multi_target(a, b)
                 ^ Lint/UnusedMethodArgument: Unused method argument - `a`.
                    ^ Lint/UnusedMethodArgument: Unused method argument - `b`.
  a, b = 1, 2
end

# block parameter shadows method parameter — method param is unused
def shadowed_by_block(x)
                      ^ Lint/UnusedMethodArgument: Unused method argument - `x`.
  items.each { |x| puts x }
end

# lambda parameter shadows method parameter — method param is unused
def shadowed_by_lambda(x)
                       ^ Lint/UnusedMethodArgument: Unused method argument - `x`.
  transform = ->(x) { x * 2 }
  transform.call(42)
end

# binding(&block) is NOT Kernel#binding — does not suppress unused arg warning
def with_binding_block_pass(bar, &blk)
                            ^^^ Lint/UnusedMethodArgument: Unused method argument - `bar`.
  binding(&blk)
end

# singleton method (def self.) with unused rest param
def self.class_method(used, *rest)
                             ^^^^ Lint/UnusedMethodArgument: Unused method argument - `rest`.
  puts used
end

# parameter only reassigned (not read) inside block — not truly used
def reassign_in_block(first, last)
                             ^^^^ Lint/UnusedMethodArgument: Unused method argument - `last`.
  items.each { |item| call(first, last = nil) }
end

# optional param only reassigned inside block body
def reassign_optional(symtab, indent = 0)
                              ^^^^^^ Lint/UnusedMethodArgument: Unused method argument - `indent`.
  children.each do |child|
    child.process(symtab, indent = 2)
  end
end
