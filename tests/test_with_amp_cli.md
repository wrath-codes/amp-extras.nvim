# Testing with Actual Amp CLI

This guide shows how to test the WebSocket server with the real Amp CLI.

## Prerequisites

- Amp CLI installed and in your PATH
- Neovim with amp-extras-rs plugin installed

## Steps

### 1. Start Neovim with the WebSocket Server

```bash
# In terminal 1
nvim
```

In Neovim, run:
```vim
:lua local result = require('amp_extras').server_start()
:lua print('Port:', result.port, 'Token:', result.token)
```

**Note the port and token displayed!**

### 2. Setup Notifications (Optional)

To receive cursor movement and file change notifications:

```vim
:lua require('amp_extras').setup_notifications()
```

### 3. Connect Amp CLI

In another terminal:

```bash
# Set environment variables (use values from step 1)
export AMP_IDE_PORT=<port>
export AMP_IDE_TOKEN=<token>

# Connect Amp CLI
amp --ide ws://127.0.0.1:${AMP_IDE_PORT}/?auth=${AMP_IDE_TOKEN}
```

Or as a one-liner:
```bash
amp --ide "ws://127.0.0.1:<port>/?auth=<token>"
```

### 4. Test Features

Once connected, Amp CLI should be able to:

#### Server-Initiated Notifications
- **Move cursor in Neovim** → Amp receives `selectionDidChange` notifications
- **Enter visual mode (v, V, Ctrl-V)** → Amp receives selection with text content
- **Open new files/splits** → Amp receives `visibleFilesDidChange` notifications

#### IDE Protocol Operations

Amp CLI can request:
- **Read files**: `readFile` method with `{"path": "/absolute/path/to/file"}`
- **Edit files**: `editFile` method with `{"path": "...", "content": "..."}`
- **Ping**: `ping` method to verify connection
- **Get diagnostics**: `getDiagnostics` method

#### JSON-RPC Format

Amp CLI uses the amp.nvim protocol format:
```json
{
  "clientRequest": {
    "id": "req-123",
    "readFile": {
      "path": "/tmp/test.txt"
    }
  }
}
```

Response:
```json
{
  "serverResponse": {
    "id": "req-123",
    "readFile": {
      "content": "file contents here"
    }
  }
}
```

### 5. Monitor Server Logs

To see what's happening on the server side:

```vim
:messages
```

Look for lines like:
- `Client X registered (total: Y)`
- `Client X unregistered (remaining: Y)`
- `Broadcast message to X clients`

### 6. Stop the Server

When done:

```vim
:lua require('amp_extras').server_stop()
```

Or just quit Neovim - the server will stop automatically.

## Troubleshooting

### Connection Refused
- Verify server is running: `:lua print(require('amp_extras').server_is_running())`
- Check firewall settings
- Verify port and token are correct

### Authentication Failed (401)
- Double-check the token matches exactly
- No extra spaces or characters in token

### No Notifications Received
- Verify notifications are setup: `:lua require('amp_extras').setup_notifications()`
- Check that autocmds are registered: `:autocmd AmpExtrasNotifications`

### Read/Write Errors
- Paths must be absolute (start with `/`)
- File must exist for `readFile`
- Parent directory must exist for `editFile`

## Example Session

```bash
# Terminal 1: Start Neovim
nvim

# In Neovim:
:lua local r = require('amp_extras').server_start()
:lua print('Port:', r.port, 'Token:', r.token:sub(1,8)..'...')
# Output: Port: 54123 Token: AbCdEfGh...

:lua require('amp_extras').setup_notifications()
# Output: {success = true}

# Terminal 2: Connect Amp
amp --ide "ws://127.0.0.1:54123/?auth=<full-token-here>"

# Now in Neovim, try:
# - Move cursor around
# - Enter visual mode and select text
# - Open new files
# - Amp CLI should receive notifications!
```
