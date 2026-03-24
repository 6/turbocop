format('foo')
^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo'` directly instead of `format`.

sprintf('bar')
^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'bar'` directly instead of `sprintf`.

Kernel.format('baz')
^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'baz'` directly instead of `format`.

format(FORMAT)
^^^^^^^^^^^^^^ Style/RedundantFormat: Use `FORMAT` directly instead of `format`.

sprintf(MSG)
^^^^^^^^^^^^ Style/RedundantFormat: Use `MSG` directly instead of `sprintf`.

format(Foo::BAR)
^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `Foo::BAR` directly instead of `format`.

format('%s %s', 'foo', 'bar')
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo bar'` directly instead of `format`.

sprintf('%-10s', 'foo')
^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo       '` directly instead of `sprintf`.

format('%d', 5)
^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'5'` directly instead of `format`.

format('%s', 'hello')
^^^^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'hello'` directly instead of `format`.

format('%s', :foo)
^^^^^^^^^^^^^^^^^^ Style/RedundantFormat: Use `'foo'` directly instead of `format`.
