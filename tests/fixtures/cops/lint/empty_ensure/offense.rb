begin
  do_something
rescue
  handle_error
ensure
^^^^^^ Lint/EmptyEnsure: Empty `ensure` block detected.
end

begin
  do_something
ensure
^^^^^^ Lint/EmptyEnsure: Empty `ensure` block detected.
end

begin
  foo
  bar
ensure
^^^^^^ Lint/EmptyEnsure: Empty `ensure` block detected.
end
