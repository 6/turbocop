begin
  foo
  rescue => e
  ^^^^^^ Layout/RescueEnsureAlignment: Align `rescue` with `begin`.
  bar
end

begin
  foo
  ensure
  ^^^^^^ Layout/RescueEnsureAlignment: Align `ensure` with `begin`.
  bar
end

begin
  baz
  rescue => e
  ^^^^^^ Layout/RescueEnsureAlignment: Align `rescue` with `begin`.
  qux
  ensure
  ^^^^^^ Layout/RescueEnsureAlignment: Align `ensure` with `begin`.
  grault
end
