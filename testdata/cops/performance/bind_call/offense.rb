foo.method(:bar).bind(obj).call
                 ^^^^ Performance/BindCall: Use `bind_call(obj)` instead of `bind(obj).call()`.
foo.method(:bar).bind(obj).call(arg1, arg2)
                 ^^^^ Performance/BindCall: Use `bind_call(obj, arg1, arg2)` instead of `bind(obj).call(arg1, arg2)`.
Foo.method(:something).bind(obj).call
                       ^^^^ Performance/BindCall: Use `bind_call(obj)` instead of `bind(obj).call()`.
umethod.bind(obj).call(foo, bar)
        ^^^^ Performance/BindCall: Use `bind_call(obj, foo, bar)` instead of `bind(obj).call(foo, bar)`.
umethod.bind(a).call
        ^^^^ Performance/BindCall: Use `bind_call(a)` instead of `bind(a).call()`.
umethod.bind(a).call(*p)
        ^^^^ Performance/BindCall: Use `bind_call(a, *p)` instead of `bind(a).call(*p)`.
CONSTANT.bind(obj).call
         ^^^^ Performance/BindCall: Use `bind_call(obj)` instead of `bind(obj).call()`.
