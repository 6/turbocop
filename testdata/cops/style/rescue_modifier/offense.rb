x = something rescue nil
    ^^^^^^^^^^^^^^^^^^^^ Style/RescueModifier: Avoid rescuing without specifying an error class.

y = foo.bar rescue false
    ^^^^^^^^^^^^^^^^^^^^ Style/RescueModifier: Avoid rescuing without specifying an error class.

z = JSON.parse(str) rescue {}
    ^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RescueModifier: Avoid rescuing without specifying an error class.
