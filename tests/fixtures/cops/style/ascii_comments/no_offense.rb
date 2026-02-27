# AZaz1@$%~,;*_`|
# Simple ascii comment
# Another comment with numbers 123
x = 1  # inline comment
y = "unicode string is fine: café"
z = 42

# Non-ASCII in string literals should not be flagged
card_label = "#{card.brand} ××#{card.last4[-2..-1]}"
html_entities = {"&#83;" => "™", "&#82;" => "€"}
greeting = "こんにちは"
msg = "Price: #{amount}€"
emoji_str = "Hello 🌍"
