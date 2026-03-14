case x
when 1
  puts 1
when 2
  puts 2
end

# Pattern matching case/in (Ruby 3.0+)
case x
in 1
  :a
in 2
  :b
end

# Single-line case/when should not be flagged
result = case x; when 1 then :a; when 2 then :b; end
val = case n; when 0 then x * 2; else y / 3; end

# Single-line case/in should not be flagged
result = case x; in 1 then :a; in 2 then :b; end
