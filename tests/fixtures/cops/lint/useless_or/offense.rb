x.to_s || fallback
       ^^^^^^^^^^^ Lint/UselessOr: `fallback` will never evaluate because `x.to_s` always returns a truthy value.
x.to_i || 0
       ^^^^ Lint/UselessOr: `0` will never evaluate because `x.to_i` always returns a truthy value.
x.inspect || 'default'
          ^^^^^^^^^^^^ Lint/UselessOr: `'default'` will never evaluate because `x.inspect` always returns a truthy value.
foo || x.to_s || fallback
              ^^^^^^^^^^^ Lint/UselessOr: `fallback` will never evaluate because `x.to_s` always returns a truthy value.
to_s || "fallback"
     ^^^^^^^^^^^^^ Lint/UselessOr: `"fallback"` will never evaluate because `to_s` always returns a truthy value.
to_i || 0
     ^^^^ Lint/UselessOr: `0` will never evaluate because `to_i` always returns a truthy value.
object.get_option('logo') || (default || h.asset_url('camaleon_cms/camaleon.png').to_s)
                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Lint/UselessOr: `(default || h.asset_url('camaleon_cms/camaleon.png').to_s)` will never evaluate because `h.asset_url('camaleon_cms/camaleon.png').to_s` always returns a truthy value.
