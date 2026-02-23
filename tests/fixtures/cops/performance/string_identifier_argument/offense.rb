obj.send('method_name')
         ^^^^^^^^^^^^^ Performance/StringIdentifierArgument: Use `:method_name` instead of `'method_name'`.
obj.respond_to?('foo')
                ^^^^^ Performance/StringIdentifierArgument: Use `:foo` instead of `'foo'`.
obj.method('bar')
           ^^^^^ Performance/StringIdentifierArgument: Use `:bar` instead of `'bar'`.
obj.public_send('baz')
                ^^^^^ Performance/StringIdentifierArgument: Use `:baz` instead of `'baz'`.
obj.define_method('my_method') { }
                  ^^^^^^^^^^^ Performance/StringIdentifierArgument: Use `:my_method` instead of `'my_method'`.
obj.instance_variable_get('@ivar')
                          ^^^^^^^ Performance/StringIdentifierArgument: Use `:@ivar` instead of `'@ivar'`.
# Command methods (receiverless)
attr_accessor 'name', 'role'
              ^^^^^^ Performance/StringIdentifierArgument: Use `:name` instead of `'name'`.
                      ^^^^^^ Performance/StringIdentifierArgument: Use `:role` instead of `'role'`.
alias_method 'new_name', 'old_name'
             ^^^^^^^^^^ Performance/StringIdentifierArgument: Use `:new_name` instead of `'new_name'`.
                         ^^^^^^^^^^ Performance/StringIdentifierArgument: Use `:old_name` instead of `'old_name'`.
private 'helper'
        ^^^^^^^^ Performance/StringIdentifierArgument: Use `:helper` instead of `'helper'`.
