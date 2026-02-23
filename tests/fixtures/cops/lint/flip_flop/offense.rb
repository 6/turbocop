(1..20).each do |x|
  puts x if (x == 5) .. (x == 10)
            ^^^^^^^^^^^^^^^^^^^^^^ Lint/FlipFlop: Avoid the use of flip-flop operators.
end

(1..20).each do |x|
  puts x if (x == 5) ... (x == 10)
            ^^^^^^^^^^^^^^^^^^^^^^^ Lint/FlipFlop: Avoid the use of flip-flop operators.
end

while (a == 1) .. (b == 2)
      ^^^^^^^^^^^^^^^^^^^^ Lint/FlipFlop: Avoid the use of flip-flop operators.
  do_something
end
