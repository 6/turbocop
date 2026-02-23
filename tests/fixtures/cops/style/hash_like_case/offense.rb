case x
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when 'a'
  1
when 'b'
  2
when 'c'
  3
end
case y
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when :foo
  'bar'
when :baz
  'qux'
when :quux
  'corge'
end
case z
^^^^ Style/HashLikeCase: Consider replacing `case-when` with a hash lookup.
when 1
  :one
when 2
  :two
when 3
  :three
end
