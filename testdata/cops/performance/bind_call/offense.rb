foo.method(:bar).bind(obj).call
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/BindCall: Use `bind_call` instead of `method.bind.call`.
foo.method(:bar).bind(obj).call(arg1, arg2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/BindCall: Use `bind_call` instead of `method.bind.call`.
Foo.method(:something).bind(obj).call
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Performance/BindCall: Use `bind_call` instead of `method.bind.call`.
