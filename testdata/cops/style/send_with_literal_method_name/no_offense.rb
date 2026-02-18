obj.public_send(method_name)
obj.__send__(var)
obj.foo
obj.bar(arg)
obj.send(:foo)
x = 1
obj.public_send("name with space")
obj.public_send("#{interpolated}string")

# AllowSend: true (default) - send and __send__ are allowed
obj.send(:literal_method)
obj.__send__(:literal_method)
generator.__send__(:snake_case, 'Foo')
