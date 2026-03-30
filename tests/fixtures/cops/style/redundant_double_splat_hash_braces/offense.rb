do_something(**{foo: bar, baz: qux})
             ^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.

method(**{a: 1})
       ^^^^^^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.

call(**{x: y, z: w})
     ^^^^^^^^^^^^^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.

self.class.new(**{ all: all, file_system: file_system, command: command }.merge(params))
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.

tag :link, **{
           ^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.
  rel: 'stylesheet',
  href: vite_asset_path(@file, type: :stylesheet),
  media: 'screen',
}.merge(@params)

described_class.new "database", **{
                                ^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.
  host: "influxdb.test",
  port: 9999,
}.merge(args)

described_class.new(
  "database",
  **{
  ^^^ Style/RedundantDoubleSplatHashBraces: Remove the redundant double splat and braces, use keyword arguments directly.
    host: "influxdb.test",
    port: 9999,
    username: "username",
    password: "password",
    time_precision: "s",
  }.merge(args)
)
