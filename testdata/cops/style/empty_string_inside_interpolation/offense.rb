"#{condition ? 'foo' : ''}"
 ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

"#{condition ? '' : 'foo'}"
 ^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.

"#{condition ? 42 : nil}"
 ^^^^^^^^^^^^^^^^^^^^^^^^ Style/EmptyStringInsideInterpolation: Do not return empty strings in string interpolation.
