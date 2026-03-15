a.func (x)
       ^ Lint/ParenthesesAsGroupedExpression: `(x)` interpreted as grouped expression.
is? (x)
    ^ Lint/ParenthesesAsGroupedExpression: `(x)` interpreted as grouped expression.
rand (1..10)
     ^ Lint/ParenthesesAsGroupedExpression: `(1..10)` interpreted as grouped expression.
a&.func (x)
        ^ Lint/ParenthesesAsGroupedExpression: `(x)` interpreted as grouped expression.
a.concat ((1..1).map { |i| i * 10 })
         ^ Lint/ParenthesesAsGroupedExpression: `((1..1).map { |i| i * 10 })` interpreted as grouped expression.
method_name (x)
            ^ Lint/ParenthesesAsGroupedExpression: `(x)` interpreted as grouped expression.
foo ("bar")
    ^ Lint/ParenthesesAsGroupedExpression: `("bar")` interpreted as grouped expression.
rand (1.to_i..10)
     ^ Lint/ParenthesesAsGroupedExpression: `(1.to_i..10)` interpreted as grouped expression.
method ({a: 1})
       ^ Lint/ParenthesesAsGroupedExpression: `({a: 1})` interpreted as grouped expression.
foo ({a: 1, b: 2})
    ^ Lint/ParenthesesAsGroupedExpression: `({a: 1, b: 2})` interpreted as grouped expression.
expect(data.row(0)).to eq ({ 'Name' => 'Mage', 'Cost' => 1 })
                          ^ Lint/ParenthesesAsGroupedExpression: `({ 'Name' => 'Mage', 'Cost' => 1 })` interpreted as grouped expression.
json.errors ({ site: current_site.errors, page: @page.errors })
            ^ Lint/ParenthesesAsGroupedExpression: `({ site: current_site.errors, page: @page.errors })` interpreted as grouped expression.
