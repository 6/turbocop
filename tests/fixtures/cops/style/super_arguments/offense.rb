def foo(a, b)
  super(a, b)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def bar(x, y)
  super(x, y)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def baz(name)
  super(name)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_rest(*args, **kwargs)
  super(*args, **kwargs)
  ^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_keyword(name:, age:)
  super(name: name, age: age)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_block(name, &block)
  super(name, &block)
  ^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

def with_mixed(a, *args, b:)
  super(a, *args, b: b)
  ^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# super() with no-arg def
def no_args
  super()
  ^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# super(...) forwarding
def with_forwarding(...)
  super(...)
  ^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Block-only forwarding
def block_only(&blk)
  super(&blk)
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# super with block literal should still flag when args match
def with_block_literal(a, b)
  super(a, b) { x }
  ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# super with block literal and non-forwarded block arg should flag
def with_block_arg_and_literal(a, &blk)
  super(a) { x }
  ^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when all positional and keyword arguments are forwarded.
end

# Ruby 3.1 hash value omission: super(type:, hash:)
def with_shorthand_keywords(type:, hash:)
  super(type:, hash:)
  ^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Nested def — inner super should flag for inner def
def outer(a)
  def inner(b:)
    super(b: b)
    ^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
  end
  super(a)
  ^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous block forwarding with named keyword rest
def with_named_kwargs_and_anonymous_block(filter: /UserAudit/, level: :info, **args, &)
  super(filter:, level:, **args, &)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous block forwarding with positional arguments
def with_args_and_anonymous_block(level, index, message, payload, exception, &)
  super(level, index, message, payload, exception, &)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous positional and keyword forwarding
def with_anonymous_rest_and_keywords(*, **)
  super(*, **) if defined?(super)
  ^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous block forwarding after local mutation
def with_mutation_before_anonymous_block(xml, &)
  xml = strip_whitespace(xml)
  super(xml, &)
  ^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Post arguments after rest params should keep Ruby's order
def with_post_arg(ep, *params, configs)
  super ep, *params, configs
  ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous block param with an inline block still reports the positional/keyword message
def with_inline_block_and_anonymous_block(name, *args, &)
  super(name, *args) do
  ^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when all positional and keyword arguments are forwarded.
    instance_eval(&)
  end
end

# Anonymous keyword rest forwarding
def with_anonymous_keyword_rest(quantity: nil, hard_limit: INFINITY, soft_limit: INFINITY, **)
  super(quantity:, hard_limit:, soft_limit:, **)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Named keyword + anonymous keyword rest forwarding
def with_named_and_anonymous_keyword_rest(io, delete: delete_raw?, **)
  super(io, delete:, **)
  ^^^^^^^^^^^^^^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end

# Anonymous keyword rest on its own
def only_anonymous_keyword_rest(**)
  super(**)
  ^^^^^^^^^ Style/SuperArguments: Call `super` without arguments and parentheses when the signature is identical.
end
