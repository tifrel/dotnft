docker run --rm --name polkadot-ui \
  -e WS_URL=ws://127.0.0.1:9944 -p 80:80 jacogr/polkadot-js-apps:latest \
  > docker.log 2>&1 &

substrate-contracts-node --dev --tmp --rpc-cors all --unsafe-rpc-external --unsafe-ws-external
