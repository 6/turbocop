puts "this is a #$test"
                 ^^^^^ Style/VariableInterpolation: Replace interpolated variable `$test` with expression `#{$test}`.

puts "this is a #@test"
                 ^^^^^ Style/VariableInterpolation: Replace interpolated variable `@test` with expression `#{@test}`.

puts "this is a #@@t"
                 ^^^ Style/VariableInterpolation: Replace interpolated variable `@@t` with expression `#{@@t}`.
