# list available recipes
default:
  @just --list --unsorted

# run tests with coverage
coverage:
  cargo +nightly llvm-cov test --no-report --all-features --doctests
  cargo +nightly llvm-cov report --doctests --html
  cargo +nightly llvm-cov report --doctests --lcov --output-path lcov.info

# upload coverage to GitHub Pages
upload-coverage: coverage
  git checkout gh-pages
  rm -rf badges src tests coverage.json index.html
  cp -R target/debug/coverage/ .
  git add -A badges src tests coverage.json index.html
  git commit -m "Coverage for $(git rev-parse --short main)"
  # git push
  git checkout main
