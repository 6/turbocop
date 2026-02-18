case x
when 'a'
  1
when 'b'
  2
end
case x
when 'a'
  do_something
when 'b'
  do_other
when 'c'
  do_third
end
LOOKUP = { 'a' => 1, 'b' => 2, 'c' => 3 }

# Case without predicate (boolean-mode case) - not flagged
case
when x == 'a'
  1
when x == 'b'
  2
when x == 'c'
  3
end
