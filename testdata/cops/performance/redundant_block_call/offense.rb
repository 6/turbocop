def method(&block)
  block.call
  ^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end

def method2(&block)
  block.call(1, 2)
  ^^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end

def method3(&block)
  block.call(arg)
  ^^^^^^^^^^^^^^^ Performance/RedundantBlockCall: Use `yield` instead of `block.call`.
end
