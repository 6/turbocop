'foo'.unpack('h*').first
^^^^^^^^^^^^^^^^^^^^^^^^ Style/UnpackFirst: Use `unpack1('h*')` instead of `'foo'.unpack('h*').first`.

'foo'.unpack('h*')[0]
^^^^^^^^^^^^^^^^^^^^^ Style/UnpackFirst: Use `unpack1('h*')` instead of `'foo'.unpack('h*')[0]`.

'foo'.unpack('h*').at(0)
^^^^^^^^^^^^^^^^^^^^^^^^ Style/UnpackFirst: Use `unpack1('h*')` instead of `'foo'.unpack('h*').at(0)`.
