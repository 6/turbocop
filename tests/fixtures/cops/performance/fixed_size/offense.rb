'a'.size
^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
"a".length
^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
:foo.size
^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
:'foo-bar'.length
^^^^^^^^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
[1, 2, foo].count
^^^^^^^^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
{a: 1, b: 2}.size
^^^^^^^^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
%w(1, 2, foo).length
^^^^^^^^^^^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
'a'&.size
^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
"a".count('o')
^^^^^^^^^^^^^^ Performance/FixedSize: Do not compute the size of statically sized objects.
