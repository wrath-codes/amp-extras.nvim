# End-to-End Test: User Messages from Neovim to Amp CLI

This guide shows how to test the new `userSentMessage` and `appendToPrompt` notifications with a real WebSocket client.

## Quick Test (Headless)

Run the automated test:

```bash
nvim --headless -u tests/user_message_test.lua
```

Expected output:
```
=== User Message Notification Tests ===
✅ All user message tests passed!
```

## Manual Test with WebSocket Client

### Step 1: Start Neovim and WebSocket Server

Open Neovim in one terminal:

```bash
nvim
```

In Neovim, start the server:

```vim
:lua require('amp_extras').server_start()
```

You'll see output like:
```
Server started on port 54321
Token: abc123defg...
Lockfile: ~/.local/share/amp/ide/54321.json
```

### Step 2: Connect WebSocket Client

In another terminal, use a WebSocket client like `wscat`:

```bash
# Install wscat if needed
npm install -g wscat

# Connect with authentication token (replace with your actual token)
wscat -c "ws://127.0.0.1:54321?token=abc123defg..."
```

You should see:
```
Connected (press CTRL+C to quit)
```

### Step 3: Send Messages from Neovim

Back in Neovim, run these Lua commands:

#### Send a message to agent:

```vim
:lua require('amp_extras').send_user_message("Explain what a WebSocket is")
```

In your WebSocket client terminal, you should see:

```json
{
  "serverNotification": {
    "userSentMessage": {
      "message": "Explain what a WebSocket is"
    }
  }
}
```

#### Append to IDE prompt:

```vim
:lua require('amp_extras').send_to_prompt("@README.md")
```

In your WebSocket client terminal, you should see:

```json
{
  "serverNotification": {
    "appendToPrompt": {
      "message": "@README.md"
    }
  }
}
```

## Practical Usage Examples

### Example 1: Send Visual Selection

Select some text in visual mode, then:

```vim
:lua local lines = vim.api.nvim_buf_get_lines(0, vim.fn.line("'<")-1, vim.fn.line("'>"), false)
:lua local text = table.concat(lines, "\n")
:lua require('amp_extras').send_user_message("Explain this code:\n\n" .. text)
```

### Example 2: Append File Reference with Line Range

```vim
:lua local file = vim.fn.expand("%:.")
:lua local line1, line2 = vim.fn.line("'<"), vim.fn.line("'>")
:lua local ref = string.format("@%s#L%d-L%d", file, line1, line2)
:lua require('amp_extras').send_to_prompt(ref)
```

### Example 3: Create User Commands

Add to your Neovim config:

```lua
-- Send message to agent
vim.api.nvim_create_user_command("AmpSend", function(opts)
  local amp = require("amp_extras")
  local result = amp.send_user_message(opts.args)
  if result and result.error then
    vim.notify("Error: " .. result.message, vim.log.levels.ERROR)
  else
    vim.notify("Message sent!", vim.log.levels.INFO)
  end
end, { nargs = "*" })

-- Append selected text to prompt
vim.api.nvim_create_user_command("AmpPromptSelection", function()
  local amp = require("amp_extras")
  
  -- Get visual selection
  local start_line = vim.fn.line("'<")
  local end_line = vim.fn.line("'>")
  local lines = vim.api.nvim_buf_get_lines(0, start_line - 1, end_line, false)
  local text = table.concat(lines, "\n")
  
  local result = amp.send_to_prompt(text)
  if result and result.error then
    vim.notify("Error: " .. result.message, vim.log.levels.ERROR)
  else
    vim.notify("Appended to prompt!", vim.log.levels.INFO)
  end
end, { range = true })

-- Append file reference to prompt
vim.api.nvim_create_user_command("AmpPromptRef", function(opts)
  local amp = require("amp_extras")
  
  local file = vim.fn.expand("%:.")
  local line1 = opts.line1
  local line2 = opts.line2
  local ref = string.format("@%s#L%d-L%d", file, line1, line2)
  
  local result = amp.send_to_prompt(ref)
  if result and result.error then
    vim.notify("Error: " .. result.message, vim.log.levels.ERROR)
  else
    vim.notify("Reference appended: " .. ref, vim.log.levels.INFO)
  end
end, { range = true })
```

Then use them:

```vim
:AmpSend What is Rust?
:AmpPromptSelection  (in visual mode)
:10,20AmpPromptRef
```

## Testing with Python WebSocket Client

For more control, use a Python script:

```python
#!/usr/bin/env python3
import asyncio
import websockets
import json
import sys

async def listen():
    # Read token from lockfile or pass as argument
    token = sys.argv[1] if len(sys.argv) > 1 else "your-token-here"
    port = sys.argv[2] if len(sys.argv) > 2 else "54321"
    
    uri = f"ws://127.0.0.1:{port}?token={token}"
    
    async with websockets.connect(uri) as websocket:
        print(f"Connected to {uri}")
        print("Listening for notifications...\n")
        
        async for message in websocket:
            data = json.loads(message)
            
            if "serverNotification" in data:
                notification = data["serverNotification"]
                
                # Handle userSentMessage
                if "userSentMessage" in notification:
                    msg = notification["userSentMessage"]["message"]
                    print(f"📨 User sent message:")
                    print(f"   {msg}\n")
                
                # Handle appendToPrompt
                elif "appendToPrompt" in notification:
                    msg = notification["appendToPrompt"]["message"]
                    print(f"✏️  Append to prompt:")
                    print(f"   {msg}\n")
                
                # Handle other notifications
                elif "selectionDidChange" in notification:
                    sel = notification["selectionDidChange"]
                    print(f"🔍 Selection changed: {sel['uri']}")
                
                elif "visibleFilesDidChange" in notification:
                    files = notification["visibleFilesDidChange"]["uris"]
                    print(f"📁 Visible files: {len(files)} files")
                
                elif "pluginMetadata" in notification:
                    meta = notification["pluginMetadata"]
                    print(f"🔌 Plugin: v{meta['version']}")

if __name__ == "__main__":
    try:
        asyncio.run(listen())
    except KeyboardInterrupt:
        print("\nDisconnected")
```

Save as `test_client.py` and run:

```bash
# Get token from lockfile
TOKEN=$(jq -r '.token' ~/.local/share/amp/ide/54321.json)
python3 test_client.py $TOKEN 54321
```

## Verification Checklist

- [ ] WebSocket server starts successfully
- [ ] Client connects with authentication token
- [ ] `send_user_message()` sends `userSentMessage` notification
- [ ] `send_to_prompt()` sends `appendToPrompt` notification
- [ ] Empty messages are handled
- [ ] Multiline messages are handled
- [ ] Special characters are preserved
- [ ] Multiple clients receive broadcasts
- [ ] Error returned when server not running

## Expected Behavior

### When server is running:
```lua
local result = amp.send_user_message("test")
-- result = { success = true }
```

### When server is NOT running:
```lua
local result = amp.send_user_message("test")
-- result = { error = true, message = "WebSocket server not running", category = "other" }
```

## Protocol Format

Both notifications follow this structure:

```json
{
  "serverNotification": {
    "methodName": {
      "message": "string content"
    }
  }
}
```

Where `methodName` is either:
- `userSentMessage` - Immediately sends message to agent
- `appendToPrompt` - Adds text to IDE prompt field (user can edit before sending)

## Troubleshooting

**"WebSocket server not running"**
- Start server with `:lua require('amp_extras').server_start()`
- Check server status with `:lua require('amp_extras').server_is_running()`

**Client can't connect**
- Check port number from server_start() output
- Verify token from lockfile matches connection string
- Check firewall isn't blocking localhost connections

**Messages not appearing in client**
- Verify client is still connected
- Check Neovim didn't crash (run command in Neovim)
- Look for Rust panics in terminal output

**JSON parsing errors**
- Check for special characters in message
- Verify message is properly escaped
- Try a simple message like "test" first
