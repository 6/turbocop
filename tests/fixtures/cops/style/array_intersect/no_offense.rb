array1.intersect?(array2)
(array1 & array2).any? { |x| false }
(array1 & array2).any?(&:block)
array1.intersection.any?
array1.intersection(array2, array3).any?
alpha & beta
array1.any? { |e| array2.include?(e) }
array1.any? { |e, i| array2.member?(e) }
array1.any? { |e| member?(e) }

# These are fine as standalone operations
(array1 & array2).size
(array1 & array2).length
(array1 & array2).count

# Size comparisons with non-zero values are not offenses
(a & b).count > 1
(a & b).count == 1
(a & b).size > 1
a.intersection(b).count > 1
a.intersection(b).count == 1
