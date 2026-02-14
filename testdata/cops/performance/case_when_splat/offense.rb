case foo
when *array
^^^^^^^^^^^^ Performance/CaseWhenSplat: Reorder `when` conditions with a splat to the end.
  bar
when 1
  baz
end
