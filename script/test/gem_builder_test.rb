#!/usr/bin/env ruby
# frozen_string_literal: true

# Run: ruby script/test/gem_builder_test.rb

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require_relative "../lib/gem_builder"

class GemBuilderTest < Minitest::Test
  def test_base_gem_assembles_correct_files
    builder = GemBuilder.new(version: "1.2.3")

    Dir.mktmpdir do |dir|
      builder.assemble(dir)

      assert File.directory?(File.join(dir, "lib")), "lib/ should exist"
      assert File.directory?(File.join(dir, "exe")), "exe/ should exist"
      refute File.directory?(File.join(dir, "libexec")), "libexec/ should NOT exist for base gem"

      assert File.file?(File.join(dir, "turbocop.gemspec")), "gemspec should exist"
      assert File.file?(File.join(dir, "lib", "turbocop.rb")), "lib/turbocop.rb should exist"
      assert File.file?(File.join(dir, "exe", "turbocop")), "exe/turbocop should exist"
    end
  end

  def test_base_gem_patches_version
    builder = GemBuilder.new(version: "1.2.3")

    Dir.mktmpdir do |dir|
      builder.assemble(dir)

      content = File.read(File.join(dir, "lib", "turbocop.rb"))
      assert_includes content, 'VERSION = "1.2.3"'
      refute_includes content, "0.0.1.pre"
    end
  end

  def test_base_gem_gemspec_has_no_platform
    builder = GemBuilder.new(version: "1.0.0")
    content = builder.gemspec_content

    refute_includes content, "spec.platform"
    assert_includes content, 'spec.name     = "turbocop"'
    assert_includes content, 'spec.authors  = ["6"]'
    assert_includes content, "runs 900+ cops."
  end

  def test_platform_gem_assembles_binary
    Dir.mktmpdir do |dir|
      # Create a fake binary
      binary = File.join(dir, "fake_turbocop")
      File.write(binary, "#!/bin/sh\necho hello\n")

      builder = GemBuilder.new(version: "2.0.0", platform: "arm64-darwin", binary_path: binary)

      out = File.join(dir, "assembled")
      FileUtils.mkdir_p(out)
      builder.assemble(out)

      assert File.directory?(File.join(out, "libexec")), "libexec/ should exist for platform gem"
      installed_bin = File.join(out, "libexec", "turbocop")
      assert File.file?(installed_bin), "binary should be copied to libexec/"
      assert File.executable?(installed_bin), "binary should be executable"
    end
  end

  def test_platform_gem_patches_version
    Dir.mktmpdir do |dir|
      binary = File.join(dir, "fake_turbocop")
      File.write(binary, "fake")

      builder = GemBuilder.new(version: "3.0.0", platform: "x86_64-linux", binary_path: binary)

      out = File.join(dir, "assembled")
      FileUtils.mkdir_p(out)
      builder.assemble(out)

      content = File.read(File.join(out, "lib", "turbocop.rb"))
      assert_includes content, 'VERSION = "3.0.0"'
    end
  end

  def test_platform_gem_gemspec_includes_platform
    builder = GemBuilder.new(version: "1.0.0", platform: "arm64-darwin", binary_path: "/dev/null")
    content = builder.gemspec_content

    assert_includes content, 'spec.platform = "arm64-darwin"'
    assert_includes content, "Platform variant: arm64-darwin."
  end

  def test_platform_gem_raises_on_missing_binary
    builder = GemBuilder.new(version: "1.0.0", platform: "arm64-darwin", binary_path: "/nonexistent/path")

    Dir.mktmpdir do |dir|
      assert_raises(RuntimeError, /Binary not found/) do
        builder.assemble(dir)
      end
    end
  end

  def test_base_gem_builds_gem_file
    builder = GemBuilder.new(version: "0.1.0")

    Dir.mktmpdir do |dir|
      dest = builder.build(output_dir: dir)

      assert File.file?(dest), ".gem file should be created"
      assert_match(/turbocop-0\.1\.0\.gem$/, dest)
    end
  end

  def test_platform_gem_builds_gem_file
    Dir.mktmpdir do |dir|
      # Create a fake binary
      binary = File.join(dir, "fake_turbocop")
      File.write(binary, "fake")

      builder = GemBuilder.new(version: "0.1.0", platform: "arm64-darwin", binary_path: binary)
      dest = builder.build(output_dir: dir)

      assert File.file?(dest), ".gem file should be created"
      assert_match(/turbocop-0\.1\.0-arm64-darwin\.gem$/, dest)
    end
  end
end
