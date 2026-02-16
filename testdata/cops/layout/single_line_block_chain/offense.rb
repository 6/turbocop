example.select { |item| item.cond? }.join('-')
                                    ^ Layout/SingleLineBlockChain: Put method call on a separate line if chained to a single line block.

example.select { |item| item.cond? }&.join('-')
                                    ^^ Layout/SingleLineBlockChain: Put method call on a separate line if chained to a single line block.

items.map { |x| x.to_s }.first
                        ^ Layout/SingleLineBlockChain: Put method call on a separate line if chained to a single line block.
