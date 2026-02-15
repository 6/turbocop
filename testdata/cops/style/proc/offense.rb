f = Proc.new { |x| x + 1 }
    ^^^^ Style/Proc: Use `proc` instead of `Proc.new`.

g = Proc.new { puts "hello" }
    ^^^^ Style/Proc: Use `proc` instead of `Proc.new`.

h = Proc.new do |x|
    ^^^^ Style/Proc: Use `proc` instead of `Proc.new`.
  x * 2
end

i = ::Proc.new { |x| x + 1 }
    ^^^^^^ Style/Proc: Use `proc` instead of `Proc.new`.
