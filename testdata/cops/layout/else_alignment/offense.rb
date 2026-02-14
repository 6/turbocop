if foo
  bar
  else
  ^^^^ Layout/ElseAlignment: Align `else` with `if`.
  baz
end

if foo
  bar
  elsif qux
  ^^^^^ Layout/ElseAlignment: Align `elsif` with `if`.
  baz
end
