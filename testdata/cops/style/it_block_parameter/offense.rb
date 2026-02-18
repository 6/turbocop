foo.each { |it| puts it }
            ^^ Style/ItBlockParameter: Avoid using `it` as a block parameter name, since `it` will be the default block parameter in Ruby 3.4+.
bar.map { |it| it.to_s }
           ^^ Style/ItBlockParameter: Avoid using `it` as a block parameter name, since `it` will be the default block parameter in Ruby 3.4+.
[1, 2].select { |it| it > 0 }
                 ^^ Style/ItBlockParameter: Avoid using `it` as a block parameter name, since `it` will be the default block parameter in Ruby 3.4+.
