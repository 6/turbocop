foo(x, **options.merge(y: 1))
         ^^^^^^^^^^^^^^^^^^^ Style/KeywordArgumentsMerging: Provide additional arguments directly rather than using `merge`.

foo(x, **options.merge(other_options))
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/KeywordArgumentsMerging: Provide additional arguments directly rather than using `merge`.

foo(x, **options.merge(y: 1, **other_options, z: 2))
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/KeywordArgumentsMerging: Provide additional arguments directly rather than using `merge`.
