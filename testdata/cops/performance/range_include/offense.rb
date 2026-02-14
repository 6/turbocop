(1..10).include?(5)
^^^^^^^^^^^^^^^^^^^ Performance/RangeInclude: Use `Range#cover?` instead of `Range#include?`.
('a'..'z').include?('m')
^^^^^^^^^^^^^^^^^^^^^^^^ Performance/RangeInclude: Use `Range#cover?` instead of `Range#include?`.
(1...100).include?(50)
^^^^^^^^^^^^^^^^^^^^^^ Performance/RangeInclude: Use `Range#cover?` instead of `Range#include?`.
