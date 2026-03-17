if foo
  bar
else
  baz
end

if foo
  bar
elsif qux
  baz
else
  quux
end

x = true ? 1 : 2

# Assignment context (keyword style): else aligns with if
links = if enabled?
          bar
        else
          baz
        end

# else/elsif correctly aligned with `if` keyword
x = if foo
      bar
    elsif qux
      baz
    else
      quux
    end

# else aligned with `if` keyword
y = if condition
      value_a
    else
      value_b
    end

# Single-line if/then/else/end — no alignment check needed
if val then puts "true" else puts "false" end
x = if cond then 'a' else 'b' end
result = defined?(if x then 'x' else '' end)

# case/when: else correctly aligned with `when`
case a
when b
  c
when d
else
  f
end

# case/when: else aligned with indented `when`
case code_type
  when 'ruby', 'sql', 'plain'
    code_type
  when 'erb'
    'ruby'
  else
    'plain'
end

# case/in: else correctly aligned with `in`
case 0
in 0
  foo
in -1..1
  bar
in Integer
  baz
else
  qux
end

# begin/rescue/else: else correctly aligned with `begin`
begin
  raise StandardError.new('Fail') if rand(2).odd?
rescue StandardError => error
  $stderr.puts error.message
else
  $stdout.puts 'Lucky you!'
end

# def/rescue/else: else correctly aligned with `def`
def my_func(string)
  puts string
rescue => e
  puts e
else
  puts e
ensure
  puts 'done'
end

# case without else — no offense
case superclass
when /\A(foo)(?:\s|\Z)/
  $1
when "self"
  namespace
end