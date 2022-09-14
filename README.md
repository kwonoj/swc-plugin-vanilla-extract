# SWC-plugin-vanilla-extract

`SWC-plugin-vanilla-extract`

`swc-plugin-vanilla-extract` is a port of `@vanilla-extract/babel-plugin` for the SWC. Transform can be performed either via SWC's wasm-based plugin, or using custom passes in rust side transform chains.

## What does compatible exactly means?

This plugin attempts to mimic most of defined behavior of original plugin's test fixture. However, due to differences of plugin envioronment it does not support few things like reading package.json's dir, or packagename. Instead it uses current working directory with configurable package name. See the test cases how does it actually works.

**NOTE: Package can have breaking changes without major semver bump**

Given SWC's plugin interface itself is under experimental stage does not gaurantee semver-based major bump yet, this package also does not gaurantee semver compliant breaking changes yet. Please refer changelogs if you're encountering unexpected breaking behavior across versions.

# Usage

## Using SWC's wasm-based experimental plugin

First, install package via npm:

```
npm install --save-dev swc-plugin-vanilla-extract
```

Then add plugin into swc's configuration:

```
const pluginOptions = { packageName?: string }

jsc: {
  ...
  experimental: {
    plugins: [
      ["swc-plugin-vanilla-extract", pluginOptions]
    ]
  }
}
```

## Using custom transform pass in rust

There is a single interface exposed to create a visitor for the transform, which you can pass into `before_custom_pass`.

```
create_extract_visitor<C: Clone + Comments, S: SourceMapper>(
    _source_map: std::sync::Arc<S>,
    _comments: C,
    filename: &str,
    package_name: &str,
    package_dir: &str,
) -> VanillaExtractVisitor
```

# Building / Testing

This package runs original plugin's fixture tests against SWC with its wasm plugin & custom transform both. `spec` contains set of the fixtures & unit test to run it, as well as supplimental packages to interop between instrumentation visitor to node.js runtime.

Few npm scripts are supported for wrapping those setups.

- `build:all`: Build all relative packages as debug build.
- `test`: Runs unit test for wasm plugin & custom transform.
- `test:debug`: Runs unit test, but only for `debug-test.yaml` fixture. This is mainly for local dev debugging for individual test fixture behavior.
