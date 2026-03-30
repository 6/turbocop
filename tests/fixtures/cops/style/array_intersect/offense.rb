(array1 & array2).any?
^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `intersect?` instead of `(array1 & array2).any?`.

(a & b).none?
^^^^^^^^^^^^^ Style/ArrayIntersect: Use `intersect?` instead of `(a & b).none?`.

a.intersection(b).any?
^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `intersect?` instead of `intersection(...).any?`.

(a & b).count > 0
^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).count > 0`.

(a & b).size > 0
^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).size > 0`.

(a & b).length > 0
^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).length > 0`.

(a & b).count == 0
^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).count == 0`.

(a & b).count != 0
^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).count != 0`.

(a & b).count.positive?
^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).count.positive?`.

(a & b).count.zero?
^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `(a & b).count.zero?`.

a.intersection(b).size > 0
^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `a.intersection(b).size > 0`.

a.intersection(b).count.positive?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `a.intersect?(b)` instead of `a.intersection(b).count.positive?`.

array1.any? { |e| array2.member?(e) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `array1.intersect?(array2)` instead of `array1.any? { |e| array2.member?(e) }`.

array1&.any? { |e| array2.member?(e) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `array1&.intersect?(array2)` instead of `array1&.any? { |e| array2.member?(e) }`.

array1.none? { array2.member?(_1) }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/ArrayIntersect: Use `!array1.intersect?(array2)` instead of `array1.none? { array2.member?(_1) }`.
