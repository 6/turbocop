obj.yield_self { |x| x.do_something }
    ^^^^^^^^^^ Style/ObjectThen: Prefer `then` over `yield_self`.

1.yield_self { |x| x + 1 }
  ^^^^^^^^^^ Style/ObjectThen: Prefer `then` over `yield_self`.

foo.yield_self(&method(:bar))
    ^^^^^^^^^^ Style/ObjectThen: Prefer `then` over `yield_self`.
