Foo.send(bar)
    ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

quuz.send(fred)
     ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

obj.send(:method_name, arg)
    ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.
