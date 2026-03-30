first_path, rest = *paths
[*paths, '/root'].each { |path| do_something(path) }
Array(paths)
[1, 2, 3]
[*a, *b]
x = [1]
other_paths = [paths] unless paths.is_a?(Array)
paths = [paths] unless paths.is_a?(Foo::Array)
# ::Array is constant_path_node — RuboCop only matches bare Array (ConstantReadNode)
paths = [paths] unless paths.is_a?(::Array)
