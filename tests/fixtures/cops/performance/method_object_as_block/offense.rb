array.map(&method(:foo))
          ^^^^^^^^^^^^^^ Performance/MethodObjectAsBlock: Use a block instead of `&method(...)` for better performance.
array.each(&method(:process))
           ^^^^^^^^^^^^^^^^^^ Performance/MethodObjectAsBlock: Use a block instead of `&method(...)` for better performance.
items.select(&method(:valid?))
             ^^^^^^^^^^^^^^^^^ Performance/MethodObjectAsBlock: Use a block instead of `&method(...)` for better performance.
