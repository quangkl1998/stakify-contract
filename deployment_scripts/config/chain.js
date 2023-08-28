"use strict";

const local = {
  rpcEndpoint: "http://localhost:26657",
  prefix: "aura",
  denom: "uaura",
  chainId: "local-aura",
  broadcastTimeoutMs: 2000,
  broadcastPollIntervalMs: 500,
};

const localDocker = {
  rpcEndpoint: "http://dev-aurad:26657",
  prefix: "aura",
  denom: "uaura",
  chainId: "local-aura",
  broadcastTimeoutMs: 2000,
  broadcastPollIntervalMs: 500,
};

const serenity = {
  rpcEndpoint: "https://rpc.serenity.aura.network",
  prefix: "aura",
  denom: "uaura",
  chainId: "serenity-testnet-001",
  broadcastTimeoutMs: 5000,
  broadcastPollIntervalMs: 1000,
};

const auraTestnet = {
  rpcEndpoint: "https://rpc.euphoria.aura.network",
  prefix: "aura",
  denom: "ueaura",
  chainId: "euphoria-2",
  broadcastTimeoutMs: 5000,
  broadcastPollIntervalMs: 1000,
};

const euphoria = {
  rpcEndpoint: "https://rpc.euphoria.aura.network",
  prefix: "aura",
  denom: "ueaura",
  chainId: "euphoria-1",
  broadcastTimeoutMs: 5000,
  broadcastPollIntervalMs: 1000,
};

const mainnet = {
  rpcEndpoint: "https://rpc.aura.network",
  prefix: "aura",
  denom: "uaura",
  chainId: "xstaxy-1",
  broadcastTimeoutMs: 5000,
  broadcastPollIntervalMs: 1000,
};

let defaultChain = auraTestnet;

defaultChain.deployer_mnemonic =
  process.env.MNEMONIC ||
  "meat hazard ivory idle grass chronic menu series alien build spoon arch organ tornado joy year earth broken brain eagle special ticket canal boy";

defaultChain.tester_mnemonic =
  process.env.MNEMONIC ||
  "meat hazard ivory idle grass chronic menu series alien build spoon arch organ tornado joy year earth broken brain eagle special ticket canal boy";

module.exports = {
  local,
  serenity,
  euphoria,
  auraTestnet,
  defaultChain,
};
