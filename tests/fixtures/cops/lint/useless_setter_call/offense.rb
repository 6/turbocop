def something
  x = Something.new
  x.attr = 5
  ^^^^^^^^^^ Lint/UselessSetterCall: Useless setter call to local variable `x`.
end

def another
  obj = Object.new
  obj.name = 'foo'
  ^^^^^^^^^^^^^^^^ Lint/UselessSetterCall: Useless setter call to local variable `obj`.
end

def third
  item = Item.new
  item.price = 100
  ^^^^^^^^^^^^^^^^ Lint/UselessSetterCall: Useless setter call to local variable `item`.
end
