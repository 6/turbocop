@some_variable ||= begin
  return some_value if some_condition_is_met
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.

  do_something
end

x = begin
  return 1
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end

@var = begin
  return :foo
  ^^^^^^ Lint/NoReturnInBeginEndBlocks: Do not `return` in `begin..end` blocks in assignment contexts.
end
