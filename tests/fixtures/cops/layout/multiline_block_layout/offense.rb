blah do |i| foo(i)
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
end

blah { |i| foo(i)
           ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
}

items.each do |x| process(x)
                  ^^^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  finalize(x)
end

blah do |i| foo(i)
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  bar(i)
rescue
  nil
end
