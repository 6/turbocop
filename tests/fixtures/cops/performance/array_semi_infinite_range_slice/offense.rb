arr[2..]
^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `drop` instead of `[]` with a semi-infinite range.
array[2..]
^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `drop` instead of `[]` with a semi-infinite range.
array[2...]
^^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `drop` instead of `[]` with a semi-infinite range.
array[..2]
^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `take` instead of `[]` with a semi-infinite range.
array[...2]
^^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `take` instead of `[]` with a semi-infinite range.
array.slice(2..)
^^^^^^^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `drop` instead of `slice` with a semi-infinite range.
array.slice(..2)
^^^^^^^^^^^^^^^^ Performance/ArraySemiInfiniteRangeSlice: Use `take` instead of `slice` with a semi-infinite range.
