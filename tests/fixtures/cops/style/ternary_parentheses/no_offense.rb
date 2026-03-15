x > 1 ? a : b

x ? a : b

foo ? bar : baz

x && y ? 1 : 0

condition ? true : false

# defined? is not complex — no parens needed
defined?(::JSON::Ext::Parser) ? ::JSON::Ext::Parser : nil
defined?(Foo) ? Foo : "fallback"
yield ? 1 : 0

# Safe assignment: indexed assignment in ternary condition
(@cache[key] = compute_value) ? true : false
