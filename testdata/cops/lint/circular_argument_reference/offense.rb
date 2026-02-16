def bake(pie: pie)
              ^^^ Lint/CircularArgumentReference: Circular argument reference - `pie`.
  pie.heat_up
end

def cook(dry_ingredients = dry_ingredients)
                           ^^^^^^^^^^^^^^^ Lint/CircularArgumentReference: Circular argument reference - `dry_ingredients`.
  dry_ingredients.reduce(&:+)
end

def greet(name: name)
                ^^^^ Lint/CircularArgumentReference: Circular argument reference - `name`.
  puts name
end
