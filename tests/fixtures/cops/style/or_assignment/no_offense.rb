x ||= 1
x = y || 1
x = x && 1
x = x + 1
name ||= 'default'
x = other ? x : 'fallback'
x = x || 1
name = name || 'default'
foo = 3 unless bar
unless foo
  bar = 3
end
unless @x
  @x = 'a'
else
  @x = 'b'
end
