arr[-1]
arr[-2]
arr[0]
arr[arr.length]
arr[other.length - 1]
foo.last
# Method-chain receivers are NOT flagged (not preserving methods)
doc.pages[doc.pages.length - 1]
assigns[:tags][assigns[:tags].length - 2]
arr[arr.method.length - 2]
arr[arr.sort.length - 2]
arr[arr.reverse.size - 2]
arr[arr.map(&:to_s).length - 2]
# Non-preserving method chains in ranges
arr[0..(arr.method.length - 2)]
# Subtraction by 0 is not flagged
arr[arr.length - 0]
# Subtraction by a variable is not flagged
arr[arr.length - n]
# Assignment form is not flagged
arr[arr.length - 2] = value
