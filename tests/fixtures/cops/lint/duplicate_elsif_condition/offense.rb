if x == 1
  :a
elsif x == 2
  :b
elsif x == 1
      ^^^^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  :c
end

if foo
  bar
elsif baz
  qux
elsif foo
      ^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  quux
end

if a > b
  1
elsif c > d
  2
elsif a > b
      ^^^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
  3
end

if get_bits(1) != 0
  bits = get_bits(4) + 2
  if get_bits(1) != 0
    delta_x = get_sbits(bits) / 20.0
    delta_y = get_sbits(bits) / 20.0
  else
    if get_bits(1) != 0
       ^^^^^^^^^^^^^^^^ Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
      delta_x = 0.0
      delta_y = get_sbits(bits) / 20.0
    else
      delta_x = get_sbits(bits) / 20.0
      delta_y = 0.0
    end
  end
end
