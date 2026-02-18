foo.map { |x| x.to_s }
          ^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:to_s` as an argument to the method instead of a block.

bar.select { |item| item.valid? }
             ^^^^^^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:valid?` as an argument to the method instead of a block.

items.reject { |i| i.nil? }
               ^^^^^^^^^^^^ Style/SymbolProc: Pass `&:nil?` as an argument to the method instead of a block.
