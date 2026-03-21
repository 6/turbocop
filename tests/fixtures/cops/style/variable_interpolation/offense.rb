puts "this is a #$test"
                 ^^^^^ Style/VariableInterpolation: Replace interpolated variable `$test` with expression `#{$test}`.

puts "this is a #@test"
                 ^^^^^ Style/VariableInterpolation: Replace interpolated variable `@test` with expression `#{@test}`.

puts "this is a #@@t"
                 ^^^ Style/VariableInterpolation: Replace interpolated variable `@@t` with expression `#{@@t}`.

options = Rubinius::Options.new "Usage: #$0 [options] [script]", 20
                                         ^^ Style/VariableInterpolation: Replace interpolated variable `$0` with expression `#{$0}`.
