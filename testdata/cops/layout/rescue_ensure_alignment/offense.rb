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
