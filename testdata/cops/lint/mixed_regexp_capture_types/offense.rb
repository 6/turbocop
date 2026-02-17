/(?<foo>bar)(baz)/
^^^^^^^^^^^^^^^^^^ Lint/MixedRegexpCaptureTypes: Do not mix named captures and numbered captures in a Regexp literal.
/(?<name>\w+)(\d+)/
^^^^^^^^^^^^^^^^^^^ Lint/MixedRegexpCaptureTypes: Do not mix named captures and numbered captures in a Regexp literal.
/(?<first>a)(b)(?<third>c)/
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/MixedRegexpCaptureTypes: Do not mix named captures and numbered captures in a Regexp literal.
