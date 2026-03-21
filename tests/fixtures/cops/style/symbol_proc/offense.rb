foo.map { |x| x.to_s }
        ^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:to_s` as an argument to the method instead of a block.

bar.select { |item| item.valid? }
           ^^^^^^^^^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:valid?` as an argument to the method instead of a block.

items.reject { |i| i.nil? }
             ^^^^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:nil?` as an argument to the method instead of a block.

# Ruby 3.4 it-block patterns
items.map { it.to_s }
          ^^^^^^^^^^^^ Style/SymbolProc: Pass `&:to_s` as an argument to the method instead of a block.

records.select { it.visible }
               ^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:visible` as an argument to the method instead of a block.

servers.any? { it.needs_recycling? }
             ^^^^^^^^^^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:needs_recycling?` as an argument to the method instead of a block.

# Numbered parameter _1 patterns
items.map { _1.to_s }
          ^^^^^^^^^^^^ Style/SymbolProc: Pass `&:to_s` as an argument to the method instead of a block.

records.select { _1.active? }
               ^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:active?` as an argument to the method instead of a block.

# super blocks (FN cases)
super { |entry| entry.call_on_yielded }
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:call_on_yielded` as an argument to `super` instead of a block.

super(headers) do |format|
               ^^^^^^^^^^^^^^ Style/SymbolProc: Pass `&:mjml` as an argument to `super` instead of a block.
  format.mjml
end
