# Reaching Ottie from other devices

Ottie listens on `127.0.0.1:24068`: only the computer it runs on can reach it. To use it from your phone, a laptop or a TV, keep it that way and let something made for the job carry the connection: [Tailscale](#tailscale-recommended), [another VPN](#another-vpn) or [a reverse proxy with HTTPS](#a-reverse-proxy-with-https). **Never open its port to the internet**: the token is a good lock, but the API isn't built to face the whole internet (no rate limits, no HTTPS of its own).

Whichever way you choose, give each device [a token of its own](#a-token-for-each-device) and [limit the folders](#which-folders-devices-may-use) devices may write to.

## Tailscale (recommended)

[Tailscale](https://tailscale.com) joins your devices into a private network (a *tailnet*). `tailscale serve` hands Ottie to that network with HTTPS, while Ottie itself stays on `127.0.0.1`:

```bash
tailscale serve --bg 24068
```

Ottie is then at `https://<computer>.<tailnet>.ts.net` for your devices only (`tailscale serve status` shows the address, `tailscale serve --https=443 off` stops it). Sign a device in with its own link (below), using that address in place of `http://127.0.0.1:24068`.

Use `serve`, never `funnel`: `tailscale funnel` puts the address on the public internet.

## Another VPN

With WireGuard, OpenVPN or your router's VPN, let Ottie listen on the VPN's address only, in `config.toml` (in Ottie's config folder; for the system-wide service, `/var/lib/ottie` or `%ProgramData%\Ottie`):

```toml
address = "10.8.0.1"   # this computer's address in the VPN
```

Restart Ottie. Devices in the VPN reach `http://10.8.0.1:24068`; the VPN encrypts the traffic. Keep the port closed to every other network in the firewall. `0.0.0.0` (every address) also opens Ottie to the local network, and to the internet if the router forwards the port: avoid it outside Docker.

## A reverse proxy with HTTPS

A reverse proxy (Caddy, nginx, Traefik…) on the same computer adds HTTPS and passes requests to Ottie on `127.0.0.1`. Keep it reachable from your own network or VPN only. Ottie needs WebSockets (live events), long requests (play links) and request bodies up to 64 MB (many links at once).

[Caddy](https://caddyserver.com) does WebSockets and HTTPS by itself:

```caddyfile
ottie.home.example {
	reverse_proxy 127.0.0.1:24068
}
```

nginx:

```nginx
server {
    listen 443 ssl;
    server_name ottie.home.example;
    # ssl_certificate …; ssl_certificate_key …;

    client_max_body_size 64m;
    location / {
        proxy_pass http://127.0.0.1:24068;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 1h;
        proxy_buffering off;
    }
}
```

Don't add password protection (HTTP Basic) in the proxy: Ottie's pages already send a token in the same `Authorization` header.

With Docker, publish the port on `127.0.0.1` only (`-p 127.0.0.1:24068:24068`, see [service.md](service.md)) and put the proxy or Tailscale in front of it.

## A token for each device

The token in Ottie's token file is **this computer's**: the desktop app, `ottie open` and scripts here use it, and only it can manage devices and folders. Every other device gets a token of its own, which you can take away without signing out the others:

- In Ottie, **Settings → Connection → Other devices**: name it, choose *Read-only* if it should only look, then **Add a device**. Its sign-in link is shown once: open it on the device, with the address it reaches Ottie at.
- Or from a terminal on this computer:

  ```bash
  ottie device add Phone           # its token and sign-in link, shown once
  ottie device add TV --read-only  # it can see, play and copy play links, nothing more
  ottie device list                # with when each was last used
  ottie device remove 2            # a lost phone: signed out at once
  ```

  (For the system-wide service: `sudo ottie device … --system`; with Docker: `docker exec ottie ottie device …`.)

A read-only device sees every page, but adds, pauses and changes nothing.

If this computer's token got out (copied into a chat, a screenshot), replace it: `ottie token --new`. The old one stops working at once; sign in again with `ottie open`, and restart the desktop app. Devices keep theirs.

## Switching between computers

Once another computer's Ottie can be reached, add it to the list in **Settings → Connection → Ottie on other computers**, with a name and a sign-in link made there for this device (above). A switcher then shows beside the logo: which Ottie this is, and the others.

- **In the desktop app,** the window shows the one you pick, and the tray, notifications and copied links follow it. "When everything has finished" stays with this computer's downloads. The app checks that it answers and takes the token before adding it, and keeps the token in the system's keychain (Credential Manager, Keychain, GNOME Keyring or KWallet). Another computer's page can't reach this one's files, programs or settings: no Open, Show in folder, folder picker or desktop settings there. This computer's Ottie keeps running meanwhile, unless **Settings → Desktop app** says to start it only once shown.
- **In a browser,** each address keeps its own list, in that browser: switching goes to the other Ottie's own page, which remembers its token (a sign-in link signs it in the first time). A bare address works too, if that page is signed in already or you have its token.

Each Ottie's page always comes from that Ottie, so two computers with different versions of Ottie work together.

## Which folders devices may use

A token can say which folder a download goes to. So that a device can't write files just anywhere this computer's account can (a Startup folder, say), devices may only use:

- each category's folder, and the folders inside it,
- the folders listed in **Settings → Connection → Folders other devices may use**,
- the folders listed in `config.toml`, which Settings can't remove:

  ```toml
  folders = ["/srv/media", "D:\\Films"]
  ```

This computer isn't limited, and only it can change categories' folders to new places or the list. A request from a device for another folder is refused.

## Checklist

- Ottie listens on `127.0.0.1` (or the VPN's address), never open to the internet.
- HTTPS between devices and Ottie: Tailscale, a VPN, or the proxy.
- One token per device; read-only where that's enough; remove the ones you no longer use (and from the lists of the computers that switch to it).
- Only the folders devices need, in Settings or `config.toml`.
- On a machine nobody sits at, the system-wide service (`ottie service install --system`, [service.md](service.md)) runs Ottie under an account of its own, which can't read people's files.
