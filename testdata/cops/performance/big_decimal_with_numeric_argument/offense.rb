BigDecimal(2.5)
           ^^^ Performance/BigDecimalWithNumericArgument: Convert float literal to string and pass it to `BigDecimal`.
BigDecimal(1.5, exception: true)
           ^^^ Performance/BigDecimalWithNumericArgument: Convert float literal to string and pass it to `BigDecimal`.
BigDecimal(3.14, 1)
           ^^^^ Performance/BigDecimalWithNumericArgument: Convert float literal to string and pass it to `BigDecimal`.
BigDecimal('1')
           ^^^ Performance/BigDecimalWithNumericArgument: Convert string literal to integer and pass it to `BigDecimal`.
BigDecimal('42', 2)
           ^^^^ Performance/BigDecimalWithNumericArgument: Convert string literal to integer and pass it to `BigDecimal`.
1.5.to_d
^^^ Performance/BigDecimalWithNumericArgument: Convert float literal to string and pass it to `BigDecimal`.
'4'.to_d
^^^ Performance/BigDecimalWithNumericArgument: Convert string literal to integer and pass it to `BigDecimal`.
