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
   unverified account cannot open a tunnel, and it fails silently. If you already
   signed up, make sure the email is confirmed.

*Why playit?* It runs a public relay that sits between your friends and your PC.
Your friends connect to the relay, the relay talks to your PC through an outbound
connection only. Your home IP is never exposed and no router setup is needed.

---

## Step 2 — Get your agent key

1. Signed in, open **https://playit.gg/account/agents/new-docker**
2. Give the agent a name (anything, e.g. `palbox`) and confirm.
3. playit shows you a **secret key** (a long line of letters and numbers).
   **Copy it.** This is what you paste into PalBox.

Keep this key private, it links the tunnel to your account.

---

## Step 3 — Create the UDP tunnel

Still on playit.gg, go to **Tunnels** and **Add / Create a tunnel**:

- **Protocol:** choose **UDP** (Palworld uses UDP). If you only see game presets,
  pick "Custom / UDP".
- **Local port:** `8211`
- **Local address / IP:** leave it as the default **`127.0.0.1`**
- **Ports:** 1 is enough.
- Pick the region closest to you and save.

That's the whole trick: the tunnel points at `127.0.0.1:8211`, which is exactly
where PalBox runs the game. You do **not** need to touch your router.

playit will now show a **public address** for this tunnel, something like
`yourname.gl.at.ply.gg:12345`. That is the address your friends will use.

---

## Step 4 — Turn it on in PalBox

1. In PalBox, open the **Overview** tab → **Safe access** panel.
2. Paste your secret key from Step 2 into the box and click **Save**.
3. Click **Turn on safe tunnel**.

PalBox starts the tunnel and, once it connects, shows the relay address right
there in the panel with a **Copy** button.

> The server must be **running** before you turn on the tunnel. If PalBox says
> "Start your server first", hit Start and wait for the green **Running** dot.

---

## Step 5 — Give friends the address

Send your friends the relay address (e.g. `yourname.gl.at.ply.gg:12345`).

In Palworld, they open **Join Palworld → connect by server address**, paste it,
enter the password if you set one, and they're in. Nothing to install on their
side.

---

## If something doesn't work

- **"Start your server first" when turning on the tunnel** — the game isn't
  running yet. On first launch it downloads for a few minutes; wait for the green
  **Running** dot, then try again.
- **Tunnel says connecting forever / friends can't join** — check that your
  playit email is verified (Step 1) and that the tunnel protocol is **UDP** with
  local address `127.0.0.1:8211` (Step 3).
- **Friends get "connection failed"** — make sure they used the full relay
  address **with the port number** at the end, and the password if you set one.
- **You changed the server port in Settings** — recreate the playit tunnel with
  the new local port.

---

That's it. Once set up, you never repeat this: turning the server on and off from
PalBox is all you need.
