obj.public_send(method_name)
obj.__send__(var)
obj.foo
obj.bar(arg)
obj.send(:foo)
x = 1
obj.public_send("name with space")
obj.public_send("#{interpolated}string")

# Setter methods (ending in =) cannot be converted to direct calls
obj.public_send(:name=, value)
obj.public_send("name=", value)
obj.send(:attribute=, new_value)

# AllowSend: true (default) - send and __send__ are allowed
obj.send(:literal_method)
obj.__send__(:literal_method)
generator.__send__(:snake_case, 'Foo')

# Method names with special characters cannot be called directly
obj.public_send(:"name-with-hyphen")
obj.public_send("{brackets}")
obj.public_send("[square_brackets]")
obj.public_send("a$b")
obj.send(:"the-name", key: "value")
obj.public_send(:"name.with.dots")

# Reserved words cannot be used as direct method calls
obj.public_send(:class)
obj.public_send(:if)
obj.public_send(:return)
obj.public_send(:end)
obj.public_send(:def)
obj.public_send(:begin)

# Operator methods cannot be converted to direct calls (obj.[](0) is unusual)
obj.public_send(:[], 0)
obj.public_send(:+, 1)
obj&.public_send(:[], 1)
obj.public_send(:[]=, 0, val)
obj.public_send(:<<, item)
obj.public_send(:==, other)
