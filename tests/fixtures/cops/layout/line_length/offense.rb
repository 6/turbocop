x = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                                                                                                                        ^^^^^ Layout/LineLength: Line is too long. [125/120]
y = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                                                                                                                        ^^^^^^^^^^ Layout/LineLength: Line is too long. [130/120]
z = "ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                                                                                                                        ^ Layout/LineLength: Line is too long. [121/120]

																				aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
                                                                                                    ^ Layout/LineLength: Line is too long. [140/120]

# A commented string concatenation like <<'taint_tracer.js...' must not open a fake heredoc.
# RuboCop still checks the later long lines.
# expect(subject.digest).to eq('pt.browser.arachni/' <<'taint_tracer.js><SCRIPT src' <<
x = [
                                                       "function( name, value ){\n            document.cookie = name + \"=post-\" + value\n        }",
                                                                                                                        ^ Layout/LineLength: Line is too long. [150/120]
]

# expect(subject.elements_with_events).to eq('pt.browser.arachni/' <<'taint_tracer.js><SCRIPT src' <<
click_handlers = {
  "click" => [
                                "function( e ) {\n\t\t\t\t// Discard the second event of a jQuery.event.trigger() and\n\t\t\t\t// when an event is called after a page has unloaded\n\t\t\t\treturn typeof jQuery !== core_strundefined && (!e || jQuery.event.triggered !== e.type) ?\n\t\t\t\t\tjQuery.event.dispatch.apply( eventHandle.elem, arguments ) :\n\t\t\t\t\tundefined;\n\t\t\t}"
                                                                                                                        ^ Layout/LineLength: Line is too long. [382/120]
  ]
}
