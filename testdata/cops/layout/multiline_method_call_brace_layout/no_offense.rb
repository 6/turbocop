foo(a,
  b)

bar(
  a,
  b
)

baz(a, b, c)

puts("hello")

qux(
  one,
  two,
  three
)

# Method call with do...end block — closing paren on same line as block end
define_method(method, &lambda do |*args, **kwargs|
  nf = nested_field
  nf ? nf.send(method, *args, **kwargs) : super(*args, **kwargs)
end)
