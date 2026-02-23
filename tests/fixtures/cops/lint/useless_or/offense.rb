x.to_s || fallback
^^^^^^^^^^^^^^^^^^ Lint/UselessOr: `fallback` will never evaluate because `x.to_s` always returns a truthy value.
x.to_i || 0
^^^^^^^^^^^ Lint/UselessOr: `0` will never evaluate because `x.to_i` always returns a truthy value.
x.inspect || 'default'
^^^^^^^^^^^^^^^^^^^^^^ Lint/UselessOr: `'default'` will never evaluate because `x.inspect` always returns a truthy value.
