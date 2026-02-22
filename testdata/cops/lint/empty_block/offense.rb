items.each { |x| }
^ Lint/EmptyBlock: Empty block detected.

items.each do |x|
^ Lint/EmptyBlock: Empty block detected.
end

foo { }
^ Lint/EmptyBlock: Empty block detected.
