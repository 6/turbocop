# turbocop-expect: 5:8 Metrics/BlockNesting: Avoid more than 3 levels of block nesting.
def baz
  while a
    if b
      if c
        while d
          z
        end
      end
    end
  end
end
