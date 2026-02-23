def bake(pie:)
  pie.refrigerate
end

def bake(pie: self.pie)
  pie.feed_to(user)
end

def cook(dry_ingredients = default_ingredients)
  dry_ingredients.combine
end

def foo(x = 1)
  x
end
