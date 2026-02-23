array1.intersect?(array2)
(array1 & array2).any? { |x| false }
(array1 & array2).any?(&:block)
array1.intersection.any?
array1.intersection(array2, array3).any?
alpha & beta

# These are fine as standalone operations
(array1 & array2).size
(array1 & array2).length
(array1 & array2).count
