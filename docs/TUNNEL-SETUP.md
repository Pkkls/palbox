# Safe access setup (playit.gg) — step by step

This is the one-time setup that lets your friends join your server **without you
opening any port on your router, and without ever sharing your real IP address**.

It takes about 5 minutes. You do it once. After that, your friends just need the
address PalBox shows them.

You only need this if you want people **outside your home network** to play. If
everyone is on the same Wi-Fi, you can skip all of this.

---

## Before you start

Create your server in PalBox first (the **Create my server** button). You can do
the tunnel setup while it downloads.

---

## Step 1 — Make a free playit account

1. Go to **https://playit.gg** and click **Sign up**. Use your email.
2. **Check your inbox and click the verification link.** This matters: an
   unverified account cannot open a tunnel, and it fails silently.

*Why playit?* It runs a public relay that sits between your friends and your PC.
Your friends connect to the relay, the relay talks to your PC through an outbound
connection only. Your home IP is never exposed and no router setup is needed.

---

## Step 2 — Get your agent key and connect it in PalBox

1. Signed in, open **https://playit.gg/account/agents/new-docker**
2. Name the agent (anything, e.g. `palbox`) and confirm. playit shows a **secret
   key** (a long line of letters/numbers). **Copy it.** Keep it private.
3. In PalBox: **Overview → Safe access** panel, paste the key and click **Save**,
   then **Turn on safe tunnel**.
4. Back on playit, open **Agents** (top menu). Your agent should now show a green
   **connected** dot. That confirms PalBox is running it. Leave it connected.

---

## Step 3 — Create the UDP tunnel

On playit, open **Tunnels → Create your first tunnel**, then fill the wizard:

1. **Name your tunnel** — anything, e.g. `PalBox Palworld`. Click **Next**.
2. **Tunnel Type** — free accounts see a "Limited Tunnel Types" notice. Type
   **`UDP`** in the search box and pick the plain **UDP** tile (the red
   **TCP+UDP** one is premium). Click **Next**.
3. **Port Count** — leave it at **1**. Click **Next**.
4. **Software Description** — type something real like **`Palworld dedicated
   server`**. Do *not* type "test" or random letters (playit can ban for that).
   Click **Next**.
5. **Usage Confirmation** — type the confirmation sentence exactly as shown
   (`I will not use this tunnel for malware, abuse, or prohibited software.`).
   Click **Next**.
6. **Public Endpoint** — pick the **Free Network** tab and click **Next**
   (routing is automatic; premium lets you pin a region).
7. **Assign to Agent** — pick the agent showing the green **connected** dot (the
   one PalBox runs). Click **Next**.
8. **Origin Config** — set **Local IP `127.0.0.1`** and **Local Port `8211`**,
   Proxy Protocol **None**. This is exactly where PalBox runs the game.
9. Click **Create Tunnel**.

playit allocates a public address after a few seconds, e.g.
`something.gl.at.ply.gg:12345`. **That is the address your friends use.**

---

## Step 4 — Share the address

Send friends the relay address (e.g. `something.gl.at.ply.gg:12345`).

In Palworld they open **Join Palworld → connect by server address**, paste it,
enter the password if you set one, and they're in. Nothing to install on their
side.

---

## If something doesn't work

- **Agent never shows "connected" on playit** — make sure you pasted the key in
  PalBox and clicked *Turn on safe tunnel*, and that your server is running.
- **"Start your server first" in PalBox** — the game isn't running yet. On first
  launch it downloads for a few minutes; wait for the green **Running** dot.
- **Tunnel created but friends can't join** — check the tunnel is **UDP**, the
  Origin is **`127.0.0.1:8211`**, and it's assigned to the **connected** agent.
- **Friends get "connection failed"** — make sure they used the full relay
  address **with the port number** at the end, and the password if you set one.
- **You changed the server port in Settings** — update the tunnel's Origin port
  to match.

---

That's it. Once set up, you never repeat this: turning the server on and off from
PalBox is all you need.
