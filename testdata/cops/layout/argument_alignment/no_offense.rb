foo(1,
    2,
    3)
bar(:a,
    :b,
    :c)
baz("x",
    "y")
single_arg(1)

# Argument after closing brace of multiline hash (not first on its line)
enum :action, {
  none: 0,
  disable: 1_000,
}, suffix: :action

# Multiple arguments on one line after a multiline arg
contain_exactly(a_hash_including({
  name: 'bar',
}), a_hash_including({
  name: 'foo',
}))
