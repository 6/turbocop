x = 'x'

y = "a"

z = 'Z'

w = "\n"

m = ?\C-\M-d

# Character literals inside regexp interpolation are ignored by RuboCop (StringHelp#on_regexp)
x2 = /#{foo.join(?,)}/
y2 = %r{#{bar.join(?|)}}
