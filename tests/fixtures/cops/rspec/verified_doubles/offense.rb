it do
  foo = double("Widget")
        ^^^^^^^^^^^^^^^^ RSpec/VerifiedDoubles: Prefer using verifying doubles over normal doubles.
end
it do
  foo = double(:widget)
        ^^^^^^^^^^^^^^^ RSpec/VerifiedDoubles: Prefer using verifying doubles over normal doubles.
end
it do
  foo = spy("Widget")
        ^^^^^^^^^^^^^ RSpec/VerifiedDoubles: Prefer using verifying doubles over normal doubles.
end
