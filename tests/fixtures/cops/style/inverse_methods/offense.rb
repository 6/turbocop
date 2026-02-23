!foo.any?
^^^^^^^^^ Style/InverseMethods: Use `none?` instead of inverting `any?`.
!foo.none?
^^^^^^^^^^ Style/InverseMethods: Use `any?` instead of inverting `none?`.
!foo.even?
^^^^^^^^^^ Style/InverseMethods: Use `odd?` instead of inverting `even?`.
!(x == false)
^^^^^^^^^^^^^ Style/InverseMethods: Use `!=` instead of inverting `==`.
items.select { |x| !x.valid? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `reject` instead of inverting `select`.
items.reject { |k, v| v != :active }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select` instead of inverting `reject`.
items.select! { |x| !x.empty? }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `reject!` instead of inverting `select!`.
items.reject! { |k, v| v != :a }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/InverseMethods: Use `select!` instead of inverting `reject!`.
