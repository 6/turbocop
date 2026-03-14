format("%s %d %i", 1, 2, 3)
format('%s %s %% %s %%%% %%%%%% %%5B', 1, 2, 3)
format(A_CONST, 1, 2, 3)
"%s %s" % [1, 2]
puts str % [1, 2]
format('%020x%+g:% g %%%#20.8x %#.0e', 1, 2, 3, 4, 5)
format("%*d", max_width, id)
format("%0*x", max_width, id)
format("%0.1f%% percent", 22.5)
format('%{y}-%{m}-%{d}', params)
format('%1$s %2$s', 'foo', 'bar')
format('%1$s %1$s', 'foo')
sprintf("%d%d", *test)
format("%d%d", *test)
format("%.d", 0)
puts "%s" % {"a" => 1}
puts "%s" % CONST
format("%%%<hex>02X", hex: 10)
"duration: %10.fms" % 42

# Heredoc format strings are skipped (RuboCop behavior)
format(<<~MSG, 1, 2, 3)
  Some message with args
MSG
sprintf(<<~MSG, 1)
  Another message
MSG

# Interpolated string with zero format fields (dstr with no %s etc.)
format("#{foo}", "bar", "baz")
"#{foo}" % [1, 2]

# Zero expected fields with array RHS (could be dynamic)
"some text" % [value]

# format/sprintf with only format string, no extra args
format("%s")
sprintf("%s %s")

# Interpolated width/precision should bail
format("%#{padding}s: %s", prefix, message)
sprintf("| %-#{key_offset}s | %-#{val_offset}s |", key, value)
Kernel.format("%.#{number_of_decimal_places}f", num)

# Format type chars not in [bBdiouxXeEfgGaAcps] are not format sequences
# Only valid types (b B d i o u x X e E f g G a A c p s) count as fields.
# %v, %n, %t, %r etc. are not valid Ruby format types.
format("%s %version", val)
format("%s %note: more", val)
"%s %result here" % [val]
'%' % []

# Annotated named format with flags/width before <name>
format("%-2<pos>d", pos: 1)
format('%06<hex>x', hex: 10)
format("%-8<loc>s", loc: "en")
format("%.3<number>d", number: 42)
format('%4<human>d', human: 100)
format('\\%06<hex>x', hex: 10)

# Numbered format with *N$ dynamic width
"%1$*2$s" % ["a", 8]
"%1$*10$s" % ["a",0,0,0,0,0,0,0,0,8]

# Custom DSL method named format with block (not Kernel#format)
format 'text/latex' do |obj|
  obj.to_s
end
format 'text/html' do |obj|
  obj.to_s
end
