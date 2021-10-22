const { ApiPromise, WsProvider, Keyring } = require("@polkadot/api");
const { CodePromise } = require("@polkadot/api-contract");
const jsonrpc = require("@polkadot/types/interfaces/jsonrpc");
const { secretPhrase } = require("./account.json");
const fs = require("fs");

const CANVAS_WS = "ws://127.0.0.1:9944";
const WASM_PATH = "./dotnft.wasm";
const METADATA_PATH = "./metadata.json";

async function initConnection() {
  const provider = new WsProvider(CANVAS_WS);
  const api = ApiPromise.create({ provider, rpc: jsonrpc });
  await api.ready;
  return api;
}

function createKey(phrase) {
  const keyring = new Keyring({ type: "sr25519" });
  const keypair = keyring.addFromMnemonic(phrase);
  return keypair;
}

async function main() {
  // console.log(jsonrpc);
  //console.log(types)
  const api = await initConnection();
  // console.log("Got API");
  const key = createKey(secretPhrase);
  // console.log("Got key");
  const abi = JSON.parse(fs.readFileSync(METADATA_PATH));
  // console.log("Got ABI");
  const wasm = fs.readFileSync(WASM_PATH);
  // console.log("Got WASM");
  const code = new CodePromise(api, abi, wasm);
  // console.log(code);
  // console.log("Got code promise");
  let blueprint;
  const unsub = await code.createBlueprint().signAndSend(key, (result) => {
    if (result.status.isInBlock || result.status.isFinalized) {
      blueprint = result.blueprint;
      unsub();
    }
  });
  // console.log(blueprint);
  console.log("Exiting");
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
