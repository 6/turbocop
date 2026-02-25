arr[1]
arr[index]
arr.first
arr.last
hash[:key]
x = [1, 2, 3]

# Chained [] — receiver is itself a [] call (hash indexing result)
params[:key][0]
hash[:items][-1]
data[:records][0]
results[:rows][-1]

# Chained [] — result of [0]/[-1] used with [] (arr[0][-1] pattern)
arr[0][-1]
items[-1][0]
records[0][:name]
