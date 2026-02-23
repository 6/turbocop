begin
  do_something
end while condition
    ^^^^^ Lint/Loop: Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.
begin
  do_something
end until condition
    ^^^^^ Lint/Loop: Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.
begin
  other_work
end while flag
    ^^^^^ Lint/Loop: Use `Kernel#loop` with `break` rather than `begin/end/while(until)`.
