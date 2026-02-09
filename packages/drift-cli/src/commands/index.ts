/**
 * Command registration — registers all 13 CLI commands.
 */

import type { Command } from 'commander';
import { registerScanCommand } from './scan.js';
import { registerCheckCommand } from './check.js';
import { registerStatusCommand } from './status.js';
import { registerPatternsCommand } from './patterns.js';
import { registerViolationsCommand } from './violations.js';
import { registerImpactCommand } from './impact.js';
import { registerSimulateCommand } from './simulate.js';
import { registerAuditCommand } from './audit.js';
import { registerSetupCommand } from './setup.js';
import { registerDoctorCommand } from './doctor.js';
import { registerExportCommand } from './export.js';
import { registerExplainCommand } from './explain.js';
import { registerFixCommand } from './fix.js';

/**
 * Register all CLI commands on the program.
 */
export function registerAllCommands(program: Command): void {
  registerScanCommand(program);
  registerCheckCommand(program);
  registerStatusCommand(program);
  registerPatternsCommand(program);
  registerViolationsCommand(program);
  registerImpactCommand(program);
  registerSimulateCommand(program);
  registerAuditCommand(program);
  registerSetupCommand(program);
  registerDoctorCommand(program);
  registerExportCommand(program);
  registerExplainCommand(program);
  registerFixCommand(program);
}
