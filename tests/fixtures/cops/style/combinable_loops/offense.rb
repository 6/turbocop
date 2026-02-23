items.each { |item| do_something(item) }
items.each { |item| do_something_else(item, arg) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.

items.each { |item| foo(item) }
items.each { |item| bar(item) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
items.each { |item| baz(item) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/CombinableLoops: Combine this loop with the previous loop.
