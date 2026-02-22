x = 'hello #{name}'
           ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

y = 'value: #{foo}'
            ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

z = '#{bar}'
     ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.

w = 'THIS. IS. #{yield.upcase}!'
               ^ Lint/InterpolationCheck: Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.
