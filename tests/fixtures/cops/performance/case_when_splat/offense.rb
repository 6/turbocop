case foo
when *array
^^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when 1
  baz
end
case foo
when *cond
^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when 4
  foobar
else
  baz
end
case foo
when *cond1
^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when *cond2
^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  doo
when 4
  foobar
else
  baz
end
case foo
when cond1, *cond2
^^^^^^^^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when cond3
  baz
end
case foo
when *cond1
^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when 8
  barfoo
when *SOME_CONSTANT
^^^^^^^^^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  doo
when 4
  foobar
else
  baz
end
case foo
when *cond
^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when *[1, 2]
  baz
end
case foo
when *Foo, Bar
^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  nil
end
