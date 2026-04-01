"#{condition ? 'foo' : ''}"
   ^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

"#{condition ? '' : 'foo'}"
   ^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

"#{condition ? 42 : nil}"
   ^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

"prefix #{
  flag ? '' : '.'
  ^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.
}"

`cmd #{flag ? 'x' : ''}`
       ^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

/#{flag ? '' : 'x'}/
   ^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

:"#{flag ? '' : 'opt_'}name"
    ^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.
