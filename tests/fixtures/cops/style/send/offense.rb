Foo.send(bar)
    ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

quuz.send(fred)
     ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

obj.send(:method_name, arg)
    ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

value = send extra_property
        ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

value = send(a)
        ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

send(__method__, value, context, convertize: convertize, reconstantize: false)
^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

send(__method__, value, context, convertize: false, reconstantize: reconstantize)
^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

send(__method__, nil)
^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

send(__method__, nil)
^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

send(self.class.paranoid_column)
^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.

associated_object = send(assoc_reflection.name)
                    ^^^^ Style/Send: Prefer `Object#__send__` or `Object#public_send` to `send`.
