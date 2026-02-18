obj.public_send(:foo)
    ^^^^^^^^^^^ Style/SendWithLiteralMethodName: Use a direct method call instead of `send` with a literal method name.

obj.__send__(:bar)
    ^^^^^^^^ Style/SendWithLiteralMethodName: Use a direct method call instead of `send` with a literal method name.

obj.public_send('baz', arg)
    ^^^^^^^^^^^ Style/SendWithLiteralMethodName: Use a direct method call instead of `send` with a literal method name.
