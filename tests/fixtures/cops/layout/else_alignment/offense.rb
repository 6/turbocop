if foo
  bar
  else
  ^^^^ Layout/ElseAlignment: Align `else` with `if`.
  baz
end

if foo
  bar
  elsif qux
  ^^^^^ Layout/ElseAlignment: Align `elsif` with `if`.
  baz
end

if alpha
  one
    else
    ^^^^ Layout/ElseAlignment: Align `else` with `if`.
  two
end

value = if condition
          one
        else
          two
        end
result = if foo
  bar
else
^^^^ Layout/ElseAlignment: Align `else` with `if`.
  baz
end

# case/when: else should align with `when`
case a
when b
  c
when d
  e
 else
 ^^^^ Layout/ElseAlignment: Align `else` with `when`.
  f
end

# case/when: else indented too far
case code_type
when 'ruby'
  code_type
when 'erb'
  'ruby'
    else
    ^^^^ Layout/ElseAlignment: Align `else` with `when`.
    'plain'
end

# case/in (pattern matching): else should align with `in`
case 0
in 0
  foo
in -1..1
  bar
in Integer
  baz
 else
 ^^^^ Layout/ElseAlignment: Align `else` with `in`.
  qux
end

# begin/rescue/else: else should align with `begin`
begin
  something
rescue
  handling
    else
    ^^^^ Layout/ElseAlignment: Align `else` with `begin`.
  fallback
end

# def/rescue/else: else should align with `def`
def my_func
  puts 'hello'
rescue => e
  puts e
  else
  ^^^^ Layout/ElseAlignment: Align `else` with `def`.
  puts 'ok'
end

# begin/rescue/else: else at column 0 should align with `begin`
def my_func
  begin
    puts 'error prone'
  rescue
    puts 'handling'
else
^^^^ Layout/ElseAlignment: Align `else` with `begin`.
    puts 'normal'
  end
end
