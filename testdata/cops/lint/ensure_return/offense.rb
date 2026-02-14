begin
  do_something
rescue
  handle_error
ensure
  return cleanup
  ^^^^^^ Lint/EnsureReturn: Do not return from an `ensure` block.
end