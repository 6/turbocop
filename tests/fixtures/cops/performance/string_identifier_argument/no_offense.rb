obj.send(:method_name)
obj.respond_to?(:foo)
obj.method(:bar)
obj.public_send(:baz)
obj.send(variable)
# Command methods with receiver are skipped
obj.attr_accessor 'something'
obj.private 'something'
obj.autoload 'something'
# String with spaces
send(':foo is :bar', foo, bar)
# String with :: (namespace)
Object.const_defined?('Foo::Bar')
# Interpolated strings
respond_to?("string_#{interpolation}")
send("do_#{action}")
# No arguments
send
# Non-string argument
send(42)
# Symbol arguments
alias_method :new, :original
attr_accessor :name, :role
