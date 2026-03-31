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

# Semicolons followed by backslash line continuation
if condition ;\
  do_something
end

begin ;\
  replace_output = @output_stream.equal?($stdout) ;\
end

# Semicolons followed by closing delimiters (RuboCop skips these)
value.each {|v| (x = v; break;) if v}
(class << self; self; end;).class_eval do
end
