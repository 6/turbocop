items.each do |x|
  puts x
end

[1, 2, 3].each { |x| puts x }

[1, 2].map do |x|
  x * 2
end

# Backslash line continuation before do — not a blank body beginning
run_command(arg1, arg2) \
  do |channel, expected|

  process(channel, expected)
end

# Lambda brace block without blank lines
action = -> (a) {
  a.map { |c| c.name }
}

# Lambda do block without blank lines
handler = -> (opts = {}) do
  opts.each { |k, v| puts v }
end
