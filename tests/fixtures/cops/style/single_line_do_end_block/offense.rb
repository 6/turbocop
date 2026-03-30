foo do |x| x end
^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

bar do puts 'hello' end
^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

baz do |a, b| a + b end
^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

foo do end
^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

foo do bar(_1) end
^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

->(arg) do foo arg end
^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

lambda do |arg| foo(arg) end
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.

# nitrocop-expect: 15:8 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
years = (years.is_a?(Array) ? years : [years])
        .sort_by do |x| x.is_a?(Range) ? x.first : x end

# nitrocop-expect: 19:2 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
assert_nothing_raised do
  (0..NUMTHREADS).map do |i|
    Thread.new do
      Thread.current.exit()
    end
  end.each do |thr| thr.join end
end

# nitrocop-expect: 26:11 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
@content = flow :hidden => true, :left => 0, :top => 0,
                :width => 1.0, :height => 1.0 do content end

# nitrocop-expect: 29:6 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
job = @scheduler
  .schedule_cron('* * * * * *', discard_past: false) do; end

# nitrocop-expect: 33:2 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
job =
  @scheduler.schedule_every(
    '7s', :first => Time.now - 60, :first_at_no_error => true
  ) do; end

# nitrocop-expect: 37:0 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
content_tag :iframe,
            src: embed_url,
            width: 560,
            height: 325,
            allow: "encrypted-media; picture-in-picture",
            allowfullscreen: true \
do alt end

# nitrocop-expect: 45:27 Style/SingleLineDoEndBlock: Prefer multiline `do`...`end` block.
enum_block = ->(yielder) { super(*args) do |item| yielder << item end }
