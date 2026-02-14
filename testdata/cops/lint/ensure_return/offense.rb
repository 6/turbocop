begin
  do_something
rescue
  handle_error
ensure
  return cleanup
  ^^^^^^ Lint/EnsureReturn: Do not return from an `ensure` block.
end
begin
  foo
ensure
  return 1
  ^^^^^^ Lint/EnsureReturn: Do not return from an `ensure` block.
end
begin
  bar
ensure
  return 2
  ^^^^^^ Lint/EnsureReturn: Do not return from an `ensure` block.
end
