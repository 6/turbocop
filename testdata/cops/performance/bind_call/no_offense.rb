foo.bind_call(obj, :bar)
foo.method(:bar)
obj.call
umethod.bind(obj).call(foo, bar)
CONSTANT.bind(obj).call
umethod.bind_call(obj, foo, bar)
