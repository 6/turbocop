str.gsub(/a+/, 'a')
^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
str.gsub(/ +/, ' ')
^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
str.gsub(/x+/, 'x')
^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
str.gsub!(/a+/, 'a')
^^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze!` instead of `gsub!`.
str.gsub(/\n+/, "\n")
^^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
str.gsub(/\t+/, "\t")
^^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
str.gsub(/\r+/, "\r")
^^^^^^^^^^^^^^^^^^^^^ Performance/Squeeze: Use `squeeze` instead of `gsub`.
