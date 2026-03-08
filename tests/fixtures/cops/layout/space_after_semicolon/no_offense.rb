x = 1; y = 2
z = "a;b"
a = 1; b = 2; c = 3
foo; bar
foo;; bar
value;	bar
puts "hello"
old_value, $; = $;, ","

def split_on_default(pattern = $;, *limit)
  text.split(pattern, *limit)
end
