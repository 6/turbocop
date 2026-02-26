%w{foo bar}
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%w`-literals should be delimited by `[` and `]`.

%i(foo bar)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%i`-literals should be delimited by `[` and `]`.

%W(cat dog)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%W`-literals should be delimited by `[` and `]`.

%r(pattern)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

%r/pattern/
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

%r/pattern/i
^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

%I(foo bar)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%I`-literals should be delimited by `[` and `]`.

%q{string}
^^^^^^^^^^ Style/PercentLiteralDelimiters: `%q`-literals should be delimited by `(` and `)`.

%Q{string}
^^^^^^^^^^ Style/PercentLiteralDelimiters: `%Q`-literals should be delimited by `(` and `)`.

%s{symbol}
^^^^^^^^^^ Style/PercentLiteralDelimiters: `%s`-literals should be delimited by `(` and `)`.

%x{command}
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%x`-literals should be delimited by `(` and `)`.

x = %r/(#{name})(\s*)(:)/
    ^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

y = %r/#{digit}+\.#{digit}+/
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

z = %r(#{name}|other)
    ^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%r`-literals should be delimited by `{` and `}`.

w = %[src="#{data_url(part)}"]
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%`-literals should be delimited by `(` and `)`.

v = %Q{value is #{items.count()}}
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%Q`-literals should be delimited by `(` and `)`.
