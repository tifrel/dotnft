# cargo +nightly contract test || exit 1
# cargo +nightly contract build || exit 1

(
  cd deploy || exit 1
  cp ../target/ink/dotnft.contract ./ || exit 1
  cp ../target/ink/dotnft.wasm ./ || exit 1
  cp ../target/ink/metadata.json ./ || exit 1
  yarn install || exit 1
  node index.js
)
