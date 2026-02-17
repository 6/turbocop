[1, 2, 3].map { |id| [id] }
          ^^^^^^^^^^^^^^^^^ Performance/ZipWithoutBlock: Use `zip` without a block argument instead.
[1, 2, 3].map { |e| [e] }
          ^^^^^^^^^^^^^^^ Performance/ZipWithoutBlock: Use `zip` without a block argument instead.
(1..3).map { |x| [x] }
       ^^^^^^^^^^^^^^^ Performance/ZipWithoutBlock: Use `zip` without a block argument instead.
[1, 2, 3].collect { |id| [id] }
          ^^^^^^^^^^^^^^^^^^^^^ Performance/ZipWithoutBlock: Use `zip` without a block argument instead.
foo.map { |id| [id] }
    ^^^^^^^^^^^^^^^^^ Performance/ZipWithoutBlock: Use `zip` without a block argument instead.
