(1..10).include?(5)
^^^^^^^^^^^^^^^^^^^ Performance/RangeInclude: Use `Range#cover?` instead of `Range#include?`.
