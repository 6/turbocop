%w{foo bar}
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%w`-literals should be delimited by `[` and `]`.

%i(foo bar)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%i`-literals should be delimited by `[` and `]`.

%W(cat dog)
^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%W`-literals should be delimited by `[` and `]`.

%{hello world}
^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%`-literals should be delimited by `(` and `)`.

%Q{#{exe} #{pars.join(' ')}}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%Q`-literals should be delimited by `(` and `)`.

%W|one #{items[0]}|
^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%W`-literals should be delimited by `[` and `]`.

%{#{func(arg)} hello}
^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%`-literals should be delimited by `(` and `)`.

%x[echo #{path.expand()}]
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%x`-literals should be delimited by `(` and `)`.

%{port: #{hash["key"]}}
^^^^^^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%`-literals should be delimited by `(` and `)`.

delimiter = %s"()<>\[\]{}/%\s"
            ^^^^^^^^^^^^^^^^^^ Style/PercentLiteralDelimiters: `%s`-literals should be delimited by `(` and `)`.
