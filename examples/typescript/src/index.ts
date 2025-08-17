#!/usr/bin/env node

// This is the main entry point for the codanna-node wrapper
// It simply delegates to the server module which handles all transport types

import { main } from './server';

// Run the server
main();
