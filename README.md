![ci badge]

# Magnolia

## Config

A `magnolia.cfg.yml` file, or whatever path is passed as argument the first argument, is required at the root of the
repository. This file contains the configuration for the bot.
The following is an example of the file structure:

`magnolia.cfg.yml`
```yaml
roles:
  devforum_member: "ROLE_ID"
  devforum_regular: "ROLE_ID"
  # Optional, allows the bot to avoid making unnecessary API calls
  roblox_verified: "ROLE_ID"
```

---

The bot also requires a `.env` file at the root of the repository. See [`.example.env`](./.example.env) for an example.

[ci badge]:https://img.shields.io/github/actions/workflow/status/archasion/discord-bot-rs/ci.yml?branch=main&event=push&label=CI