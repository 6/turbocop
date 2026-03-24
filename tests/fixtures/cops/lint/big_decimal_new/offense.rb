BigDecimal.new(123.456, 3)
           ^^^ Lint/BigDecimalNew: `BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.
BigDecimal.new('123')
           ^^^ Lint/BigDecimalNew: `BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.
::BigDecimal.new(1)
             ^^^ Lint/BigDecimalNew: `BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.
x = BigDecimal.new('3.14')
               ^^^ Lint/BigDecimalNew: `BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.
result = BigDecimal.new(amount, precision)
                    ^^^ Lint/BigDecimalNew: `BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.
