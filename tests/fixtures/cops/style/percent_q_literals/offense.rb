%Q(hello world)
^^^^^^^^^^^^^^^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q[foo bar]
^^^^^^^^^^^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q{test string}
^^^^^^^^^^^^^^^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q(escaped\\backslash)
^^^^^^^^^^^^^^^^^^^^^^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(values_for("<<HEREDOC\n\n1\nHEREDOC")).to eq  [[%Q'"\\n1\\n"'], [], [], []] # newlines escaped b/c lib inspects them
                                                       ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(values_for("<<-HEREDOC\n\n1\nHEREDOC")).to eq [[%Q'"\\n1\\n"'], [], [], []]
                                                       ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

let(:args) { ['-W', '--showformat', %Q{'${Status} ${Package} ${Version}\\n'}] }
                                    ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

let(:args_with_provides) { ['/bin/dpkg-query','-W', '--showformat', %Q{'${Status} ${Package} ${Version} [${Provides}]\\n'}]}
                                                                    ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(provider).to receive(:dpkgquery).with('-W', '--showformat', %Q{'${Status} ${Package} ${Version} [${Provides}]\\n'}).and_return(query_output)
                                                                   ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(provider).to receive(:dpkgquery).with('-W', '--showformat', %Q{'${Status} ${Package} ${Version}\\n'}, resource_name).and_return("#{dpkg_query_result} #{resource_name}")
                                                                   ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(provider).to receive(:dpkgquery).with('-W', '--showformat', %Q{'${Status} ${Package} ${Version} [${Provides}]\\n'}).and_return(query_output)
                                                                   ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

expect(provider).to receive(:dpkgquery).with('-W', '--showformat', %Q{'${Status} ${Package} ${Version}\\n'}, resource_name).and_return("#{dpkg_query_result} #{resource_name}")
                                                                   ^ Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q{Does anyone have some programs to share?
}
# nitrocop-expect: 25:0 Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q{I've done all the Lessons! What should I check out next?
}
# nitrocop-expect: 28:0 Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.

%Q{What is 'ruby on rails?' I've seen stuff about it when I google for Ruby.
}
# nitrocop-expect: 31:0 Style/PercentQLiterals: Do not use `%Q` unless interpolation is needed. Use `%q`.
