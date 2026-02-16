{ key: 'value' }.dig(:key)
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleArgumentDig: Use `{ key: 'value' }[:key]` instead of `{ key: 'value' }.dig(:key)`.

[1, 2, 3].dig(0)
^^^^^^^^^^^^^^^^ Style/SingleArgumentDig: Use `[1, 2, 3][0]` instead of `[1, 2, 3].dig(0)`.

hash.dig(:key)
^^^^^^^^^^^^^^ Style/SingleArgumentDig: Use `hash[:key]` instead of `hash.dig(:key)`.
