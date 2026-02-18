array1.intersect?(array2)
(array1 & array2).any? { |x| false }
(array1 & array2).any?(&:block)
array1.intersection.any?
array1.intersection(array2, array3).any?
alpha & beta
