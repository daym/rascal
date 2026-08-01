# Extracted Pascal test corpus

The files below `generated/` are decoded from complete Pascal programs, units,
libraries, packages, and bare program bodies embedded in `tests/*.cc`.

Regenerate them with:

```sh
cargo run --bin extract_pascal_tests
```

Check that the committed corpus is current without writing files:

```sh
cargo run --bin extract_pascal_tests -- --check
```

`generated/manifest.tsv` records the original C++ file, line, enclosing test
function, compilation-unit kind, and decoded byte length for every fixture.
The extractor owns only `tests/pascal/generated`; hand-written Pascal tests
should be placed elsewhere below `tests/pascal`.

The extracted `.pp` files preserve the decoded C++ string contents exactly.
They do not contain generated provenance comments because those would alter
source locations and parser behavior.
