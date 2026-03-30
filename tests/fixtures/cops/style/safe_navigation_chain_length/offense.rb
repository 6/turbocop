x&.foo&.bar&.baz
^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

a&.b&.c&.d
^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

user&.address&.city&.name
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

@names = names&.sort_by(&:last)&.to_h&.transform_values { |v| new(v) }
         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

params[:view_token] || session['view_token']&.[](record&.id&.to_s)
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

params[:view_token] || session['view_token']&.[](record.current_result&.id&.to_s)
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

s&.split('.')&.map(&:to_i)&.extend(Comparable)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

got = called&.args&.map(&:inspect)&.join(', ')
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

assert_equal('expected', result.err&.lines&.map(&:rstrip)&.join("\n"))
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

@object&.errors&.map(&:attribute)&.include?(attribute.to_sym)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.

cipher.iv = @options[:initiator_vector]&.split('')&.map(&:to_i)&.pack('c*')
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigationChainLength: Avoid safe navigation chains longer than 2 calls.
