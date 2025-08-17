#!/usr/bin/env node

import spawn from 'cross-spawn';
import express, { Request, Response } from 'express';
import { resolve } from 'path';
import { existsSync, readFileSync } from 'fs';
import { randomUUID } from 'crypto';
import { createServer as createHttpServer } from 'http';
import { createServer as createHttpsServer } from 'https';

/**
 * Transport configuration
 */
interface TransportConfig {
  type: 'stdio' | 'sse';
  port?: number;
  host?: string;
  sslCert?: string;
  sslKey?: string;
  sslCa?: string;
}

/**
 * Find the Codanna binary in the system
 */
function findCodannaBinary(): string | null {
  const envPath = process.env.CODANNA_PATH;
  if (envPath && existsSync(envPath)) {
    return envPath;
  }

  const which = spawn.sync('which', ['codanna'], { stdio: 'pipe' });
  if (which.status === 0 && which.stdout) {
    const binaryPath = which.stdout.toString().trim();
    if (existsSync(binaryPath)) {
      return binaryPath;
    }
  }

  const homeDir = process.env.HOME || process.env.USERPROFILE;
  if (homeDir) {
    const cargoBinPath = resolve(homeDir, '.cargo', 'bin', 'codanna');
    if (existsSync(cargoBinPath)) {
      return cargoBinPath;
    }
  }

  return null;
}

/**
 * Start stdio server (for Claude Desktop)
 */
export function startStdioServer(codannaBinary: string, codannaArgs: string[] = []): void {
  console.error(`[codanna-node] Starting stdio transport`);
  console.error(`[codanna-node] Using Codanna at: ${codannaBinary}`);

  // Default args if none provided
  const args = codannaArgs.length > 0 ? codannaArgs : ['serve', '--watch'];
  console.error(`[codanna-node] Codanna args: ${args.join(' ')}`);

  const child = spawn(codannaBinary, args, {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: process.env,
  });

  if (!child.stdin || !child.stdout || !child.stderr) {
    console.error('[codanna-node] Error: Child process stdio streams not available');
    process.exit(1);
  }

  // Set up bidirectional stdio proxying
  process.stdin.pipe(child.stdin);
  child.stdout.pipe(process.stdout);
  child.stderr.pipe(process.stderr);

  // Handle stream errors
  process.stdin.on('error', (err) => {
    console.error('[codanna-node] stdin error:', err);
  });

  child.stdin.on('error', (err) => {
    console.error('[codanna-node] child stdin error:', err);
  });

  child.stdout.on('error', (err) => {
    console.error('[codanna-node] child stdout error:', err);
  });

  child.stderr.on('error', (err) => {
    console.error('[codanna-node] child stderr error:', err);
  });

  // Handle graceful shutdown
  const shutdown = (signal: string): void => {
    console.error(`[codanna-node] Received ${signal}, shutting down...`);
    if (child.stdin && !child.stdin.destroyed) {
      child.stdin.end();
    }
    child.kill();
    process.exit(0);
  };

  process.on('SIGINT', () => shutdown('SIGINT'));
  process.on('SIGTERM', () => shutdown('SIGTERM'));

  child.on('exit', (code: number | null, signal: NodeJS.Signals | null) => {
    if (signal) {
      console.error(`[codanna-node] Codanna process killed with signal ${signal}`);
    } else {
      console.error(`[codanna-node] Codanna process exited with code ${code}`);
    }
    process.exit(code ?? 1);
  });

  child.on('error', (err: Error) => {
    console.error('[codanna-node] Failed to start Codanna:', err);
    process.exit(1);
  });

  process.stdin.on('end', () => {
    console.error('[codanna-node] Parent stdin closed, shutting down...');
    shutdown('STDIN_CLOSE');
  });
}

/**
 * Start SSE server (for remote MCP access via Messages API)
 * This creates an HTTP server that bridges SSE/HTTP to Codanna's stdio
 */
export function startSSEServer(
  codannaBinary: string,
  config: TransportConfig,
  codannaArgs: string[] = [],
): void {
  const port = config.port || 3000;
  const host = config.host || '0.0.0.0';

  // Check for SSL configuration
  const useSSL = config.sslCert && config.sslKey;
  const protocol = useSSL ? 'https' : 'http';

  // Load SSL certificates if provided
  const sslOptions = useSSL
    ? {
        cert: readFileSync(config.sslCert!),
        key: readFileSync(config.sslKey!),
        ...(config.sslCa ? { ca: readFileSync(config.sslCa) } : {}),
      }
    : {};

  console.error(`[codanna-node] Starting SSE transport on ${protocol}://${host}:${port}`);
  console.error(`[codanna-node] Using Codanna at: ${codannaBinary}`);
  if (useSSL) {
    console.error(`[codanna-node] SSL enabled with certificate: ${config.sslCert}`);
  }

  // Also respect NODE_EXTRA_CA_CERTS environment variable
  if (process.env.NODE_EXTRA_CA_CERTS) {
    console.error(`[codanna-node] Using additional CA certificates from NODE_EXTRA_CA_CERTS`);
  }

  // Create Express app
  const app = express();
  app.use(express.json());

  // Store active sessions
  interface Session {
    id: string;
    child: ReturnType<typeof spawn>;
    res?: express.Response;
    messageBuffer: string;
  }
  const sessions = new Map<string, Session>();

  // SSE endpoint - establishes connection
  app.get('/sse', (req: Request, res: Response) => {
    const sessionId = randomUUID();
    console.error(`[codanna-node] New SSE connection: ${sessionId}`);

    // Spawn new Codanna instance for this session
    // Use provided args or default to ['serve', '--watch']
    const args = codannaArgs.length > 0 ? codannaArgs : ['serve', '--watch'];
    const child = spawn(codannaBinary, args, {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: process.env,
    });

    if (!child.stdin || !child.stdout || !child.stderr) {
      res.status(500).json({ error: 'Failed to start Codanna process' });
      return;
    }

    // Set up SSE headers
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
      'Access-Control-Allow-Origin': '*',
      'X-Session-Id': sessionId,
    });

    // Create session
    const session: Session = {
      id: sessionId,
      child,
      res,
      messageBuffer: '',
    };
    sessions.set(sessionId, session);

    // Send session ID to client
    res.write(`data: ${JSON.stringify({ sessionId })}\n\n`);

    // Forward Codanna stdout to SSE
    child.stdout.on('data', (data) => {
      const text = data.toString();
      session.messageBuffer += text;

      // Try to parse complete JSON messages
      const lines = session.messageBuffer.split('\n');
      session.messageBuffer = lines[lines.length - 1] || ''; // Keep incomplete line

      for (let i = 0; i < lines.length - 1; i++) {
        const line = lines[i]?.trim();
        if (line) {
          try {
            // Validate it's JSON
            JSON.parse(line);
            res.write(`data: ${line}\n\n`);
          } catch {
            // Not JSON, skip
          }
        }
      }
    });

    // Forward Codanna stderr to console
    child.stderr.on('data', (data) => {
      console.error(`[codanna-node][${sessionId}] ${data.toString().trim()}`);
    });

    // Handle child process exit
    child.on('exit', (code) => {
      console.error(`[codanna-node][${sessionId}] Codanna exited with code ${code}`);
      sessions.delete(sessionId);
      if (!res.writableEnded) {
        res.end();
      }
    });

    // Handle client disconnect
    req.on('close', () => {
      console.error(`[codanna-node][${sessionId}] Client disconnected`);
      if (child.stdin && !child.stdin.destroyed) {
        child.stdin.end();
      }
      child.kill();
      sessions.delete(sessionId);
    });
  });

  // Message endpoint - receives messages from client
  app.post('/message', (req: Request, res: Response) => {
    const sessionId = req.headers['x-session-id'] as string;

    if (!sessionId) {
      res.status(400).json({ error: 'Missing X-Session-Id header' });
      return;
    }

    const session = sessions.get(sessionId);
    if (!session) {
      res.status(404).json({ error: 'Session not found' });
      return;
    }

    // Forward message to Codanna
    const message = JSON.stringify(req.body);
    session.child.stdin?.write(message + '\n');

    res.json({ status: 'ok' });
  });

  // Health check endpoint
  app.get('/health', (_req: Request, res: Response) => {
    res.json({
      status: 'ok',
      sessions: sessions.size,
      transport: 'sse',
    });
  });

  // CORS preflight
  app.options('*', (_req: Request, res: Response) => {
    res.header('Access-Control-Allow-Origin', '*');
    res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.header('Access-Control-Allow-Headers', 'Content-Type, X-Session-Id');
    res.sendStatus(204);
  });

  // Create HTTP or HTTPS server
  const server = useSSL ? createHttpsServer(sslOptions, app) : createHttpServer(app);

  // Start server
  server.listen(port, host, () => {
    console.error(`[codanna-node] SSE server listening on ${protocol}://${host}:${port}`);
    console.error(`[codanna-node] Endpoints:`);
    console.error(`  - SSE stream: GET ${protocol}://${host}:${port}/sse`);
    console.error(`  - Send message: POST ${protocol}://${host}:${port}/message`);
    console.error(`  - Health check: GET ${protocol}://${host}:${port}/health`);
  });

  // Handle shutdown
  const shutdown = (signal: string): void => {
    console.error(`[codanna-node] Received ${signal}, shutting down...`);

    // Kill all sessions
    for (const session of sessions.values()) {
      if (session.child.stdin && !session.child.stdin.destroyed) {
        session.child.stdin.end();
      }
      session.child.kill();
      session.res?.end();
    }

    server.close(() => {
      process.exit(0);
    });
  };

  process.on('SIGINT', () => shutdown('SIGINT'));
  process.on('SIGTERM', () => shutdown('SIGTERM'));
}

/**
 * Parse command line arguments and environment variables
 */
function parseArgs(): TransportConfig & {
  codannaPath?: string;
  help?: boolean;
  codannaArgs?: string[];
} {
  const args = process.argv.slice(2);
  const config: TransportConfig & { codannaPath?: string; help?: boolean; codannaArgs?: string[] } =
    {
      type: 'stdio', // default
      codannaArgs: [],
    };

  // Load SSL configuration from environment variables if not provided via CLI
  // This allows users to set these in their shell profile or .env file
  if (process.env.CODANNA_SSL_CERT) {
    config.sslCert = process.env.CODANNA_SSL_CERT;
  }
  if (process.env.CODANNA_SSL_KEY) {
    config.sslKey = process.env.CODANNA_SSL_KEY;
  }
  if (process.env.CODANNA_SSL_CA) {
    config.sslCa = process.env.CODANNA_SSL_CA;
  }

  let collectingCodannaArgs = false;

  for (let i = 0; i < args.length; i++) {
    // Double dash (--) indicates all following args should be passed to Codanna
    if (args[i] === '--') {
      collectingCodannaArgs = true;
      continue;
    }

    // If we're collecting Codanna args, add them to the array
    if (collectingCodannaArgs) {
      config.codannaArgs!.push(args[i]!);
      continue;
    }

    switch (args[i]) {
      case '--transport':
        const transport = args[++i];
        if (transport === 'stdio' || transport === 'sse') {
          config.type = transport as 'stdio' | 'sse';
        } else {
          console.error(`Invalid transport: ${transport}`);
          process.exit(1);
        }
        break;
      case '--port':
        config.port = parseInt(args[++i] || '3000', 10);
        break;
      case '--host':
        config.host = args[++i] || '0.0.0.0';
        break;
      case '--codanna-path':
        config.codannaPath = args[++i] || '';
        break;
      case '--ssl-cert':
        config.sslCert = args[++i] || '';
        break;
      case '--ssl-key':
        config.sslKey = args[++i] || '';
        break;
      case '--ssl-ca':
        config.sslCa = args[++i] || '';
        break;
      case '--help':
        config.help = true;
        break;
      default:
        // Unknown argument - could be a Codanna argument
        // Add it to codannaArgs if it starts with a dash
        if (args[i]?.startsWith('-')) {
          config.codannaArgs!.push(args[i]!);
        }
        break;
    }
  }

  return config;
}

/**
 * Main entry point
 */
export function main(): void {
  const config = parseArgs();

  if (config.help) {
    console.log(`
codanna-node - Node.js wrapper for Codanna MCP server

Usage: codanna-node [options]

Options:
  --transport <type>    Transport type: 'stdio' (default) or 'sse'
  --port <number>       Port for SSE server (default: 3000)
  --host <string>       Host for SSE server (default: 0.0.0.0)
  --codanna-path <path> Path to Codanna binary
  --ssl-cert <path>     Path to SSL certificate file (for HTTPS)
  --ssl-key <path>      Path to SSL private key file (for HTTPS)
  --ssl-ca <path>       Path to SSL CA certificate file (optional)
  --help                Show this help message
  --                    All arguments after this are passed to Codanna

Examples:
  # stdio transport (for Claude Desktop)
  codanna-node

  # Pass custom arguments to Codanna
  codanna-node -- serve --watch
  codanna-node -- serve --no-watch
  codanna-node -- serve --watch --port 8080

  # SSE transport with HTTP (for remote MCP access)
  codanna-node --transport sse --port 3000
  
  # SSE transport with HTTPS (for secure remote access)
  codanna-node --transport sse --port 3443 \
    --ssl-cert /path/to/cert.pem \
    --ssl-key /path/to/key.pem \
    --ssl-ca /path/to/ca.pem
  
  # SSE with custom Codanna arguments
  codanna-node --transport sse --port 3000 -- serve --no-watch

Environment Variables:
  CODANNA_PATH          Path to Codanna binary
  CODANNA_SSL_CERT      Path to SSL certificate file (for HTTPS)
  CODANNA_SSL_KEY       Path to SSL private key file (for HTTPS)
  CODANNA_SSL_CA        Path to SSL CA certificate file (optional)
  NODE_EXTRA_CA_CERTS   Path to additional CA certificates (built-in Node.js feature)
`);
    process.exit(0);
  }

  const codannaBinary = config.codannaPath || findCodannaBinary();

  if (!codannaBinary) {
    console.error('Error: Codanna binary not found.');
    console.error('Please install Codanna or set CODANNA_PATH environment variable.');
    process.exit(1);
  }

  // Start appropriate transport
  if (config.type === 'sse') {
    startSSEServer(codannaBinary, config, config.codannaArgs);
  } else {
    startStdioServer(codannaBinary, config.codannaArgs);
  }
}

// Handle unhandled promise rejections
process.on('unhandledRejection', (reason, promise) => {
  console.error('[codanna-node] Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

// Run if called directly
if (require.main === module) {
  main();
}
