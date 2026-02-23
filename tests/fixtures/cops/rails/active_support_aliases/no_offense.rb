'some_string'.start_with?('prefix')
'some_string'.end_with?('suffix')
[1, 2, 3] << 4
[1, 2, 3].unshift(0)
name.start_with?("Dr.")
path.end_with?(".rb")
# Non-literal receivers should not be flagged
str.starts_with?("foo")
str.ends_with?("bar")
variable.starts_with?("prefix")
(params[:id] || "").ends_with?(".json")
some_array.append(item)
some_array.prepend(item)
