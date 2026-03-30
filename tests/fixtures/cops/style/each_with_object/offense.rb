[].inject({}) { |a, e| a }
   ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.

[].reduce({}) { |a, e| a }
   ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.

[1, 2, 3].inject({}) do |h, i|
          ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.
  h[i] = i
  h
end

params_ordered.reduce(ActiveSupport::OrderedHash.new) { |h, p| h[p.name] = p; h }
               ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.

inject(effective_cookies) do |h, cookie|
^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.
  h[cookie.name] ||= cookie.value
  h
end

@raw_cookies.inject(effective_cookies) do |h, cookie|
             ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.
  h[cookie.raw_name] ||= cookie.raw_value
  h
end

self.class.defaults.reduce(ActiveSupport::HashWithIndifferentAccess.new) do |hash, (option, default)|
                    ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.
  hash[option] = send(option)
  hash
end

locked_dependencies.reduce(Molinillo::DependencyGraph.new) do |graph, dep|
                    ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.
  graph.add_vertex(dep.name, dep, true)
  graph
end

candidates = candidates.inject(Hash.new(0)) { |h, i| h[i] += 1; h }
                        ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.

reduce(Cleaners.new) do |cleaners, (key, value)|
^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `reduce`.
  cleaners[key] = value
  cleaners
end

platform_versions.inject(hash) do |hash, part|
                  ^^^^^^ Style/EachWithObject: Use `each_with_object` instead of `inject`.
  hash[part.split(" ")[1..-1]] = part.split(" ").first
  hash
end
