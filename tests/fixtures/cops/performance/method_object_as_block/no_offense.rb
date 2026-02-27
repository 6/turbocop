array.map { |x| foo(x) }
array.map(&:foo)
array.each { |x| process(x) }
array.map(&block)
method(:foo)
super(&method(:convert_value))
super(&method(:convert_key))
# Variable argument — not a symbol literal
array.map(&method(some_variable))
items.each(&method(handler_name))
context.define_singleton_method(k, &method(v))
items.transform_values(&method(path_method))
# String argument — not a symbol literal
array.map(&method("string_arg"))
# Safe navigation call — RuboCop ^send excludes csend
result&.then(&method(:parse))
result&.then(&JSON.method(:parse))
