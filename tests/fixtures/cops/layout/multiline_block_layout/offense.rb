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

# Lambda with body on same line as opening brace
html = -> { content
            ^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  more_content
}

# Lambda with params and body on same line
transform = ->(x) { x + 1
                    ^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  y = x * 2
}

# Lambda do..end with body on same line
action = -> do run_task
               ^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  cleanup
end

# Lambda with heredoc body on same line as opening brace
render -> { <<~HTML
            ^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
<p>hello</p>
HTML
}

# Lambda with method call body on same line
process = -> { transform(data)
               ^^^^^^^^^^^^^^^ Layout/MultilineBlockLayout: Block body expression is on the same line as the block start.
  finalize(data)
}
