# Deployment runbook

Prerequisites: an Ubuntu 24.04 host, a domain with `weather` A/AAAA records pointed at it, and inbound TCP 80/443 open.

1. Install Node.js 22, nginx, and certbot with its nginx plugin.
2. Create the system account: `sudo useradd --system --home /opt/isobar --shell /usr/sbin/nologin isobar`.
3. Copy the repository to `/opt/isobar`, run `npm ci --omit=dev`, and give `isobar:isobar` ownership.
4. Copy `deploy/isobar.service` to `/etc/systemd/system/`.
5. Replace `weather.example.com` in `deploy/nginx.conf`, copy it to `/etc/nginx/sites-available/isobar`, enable it, and validate with `sudo nginx -t`.
6. Run `sudo systemctl daemon-reload && sudo systemctl enable --now isobar && sudo systemctl reload nginx`.
7. After `weather.isobars.xyz` resolves to `54.154.121.30`, run `sudo certbot --nginx -d weather.isobars.xyz`.
8. Verify locally, then from a different network:

   `curl --fail --show-error https://weather.isobars.xyz/health`

   `curl --get --data-urlencode 'q=Kraków' https://weather.isobars.xyz/weather`

Do not proceed to Telegraph registration until the external health request succeeds and returns `upstream_ok: true`.
