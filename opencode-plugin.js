import { execFileSync } from "node:child_process";

export const SafeChains = async () => ({
  "tool.execute.before": async (input, output) => {
    if (input.tool === "bash") {
      try {
        execFileSync("safe-chains", [output.args.command]);
      } catch {
        throw new Error("Command blocked by safe-chains: not in the read-only allowlist. See https://github.com/michaeldhopkins/safe-chains for supported commands.");
      }
    }
  }
})
