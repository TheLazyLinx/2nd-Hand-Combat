import { runTool } from './tooling.mjs';
const [command, ...args] = process.argv.slice(2);
if (!command) throw new Error('Usage: node scripts/tool.mjs <tool> [arguments]');
runTool(command, args);
