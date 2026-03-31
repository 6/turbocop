a + b * c
    ^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a || b && c
     ^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a ** b + c
^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a && b * c
     ^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a * b && c
^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a || b + c
     ^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a << b || c
^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a && b | c
     ^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a and b or c
^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

x and y or z and w
^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

a && b || c
^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

html.<<("  " * (l.length-1)) unless l[-2] =~ /[ +-]*pre\/$/
        ^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

html.<<("  " * (l.length-1)) unless parent =~ /[ +-]*pre\/$/
        ^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

Sequel.|(foo, ds.where(object_id: nil).exists & {project_id => Sequel[from][:project_id]})
              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

div(span, unit).then { |res| [res, to.+(res * span, unit).unwrap] }
                                        ^^^^^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.

self.+(span * 7, :day)
       ^^^^^^^^ Lint/AmbiguousOperatorPrecedence: Wrap expressions with varying precedence with parentheses to avoid ambiguity.
