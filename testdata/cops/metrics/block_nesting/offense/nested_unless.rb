# rblint-expect: 5:8 Metrics/BlockNesting: Avoid more than 3 levels of block nesting.
def bar
  unless a
    if b
      unless c
        unless d
          y
        end
      end
    end
  end
end
