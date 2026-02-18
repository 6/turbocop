collection.each do
^^^^^^^^^^^^^^^^^^ Style/NumberedParameters: Avoid using numbered parameters for multi-line blocks.
  puts _1
end

items.map do
^^^^^^^^^^^^ Style/NumberedParameters: Avoid using numbered parameters for multi-line blocks.
  _1.to_s
end

data.select do
^^^^^^^^^^^^^^ Style/NumberedParameters: Avoid using numbered parameters for multi-line blocks.
  _1 > 0
end
