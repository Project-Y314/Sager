pub fn get_dashboard_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Sager</title>
    <link rel="icon" type="image/png" href="/api/logo">
    <style>
        :root {
            --bg: #000;
            --card: #111;
            --card-hover: #181818;
            --border: #333;
            --border-soft: #222;
            --text: #fff;
            --muted: #9a9a9a;
            --ok: #4ade80;
            --warn: #facc15;
            --bad: #f87171;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
            font-family: 'Segoe UI', system-ui, sans-serif;
            -webkit-font-smoothing: antialiased;
        }

        body {
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
            font-size: 14px;
        }

        .wrap {
            max-width: 860px;
            margin: 0 auto;
            padding: 24px 20px 80px;
        }

        /* ── Header ── */
        header {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 20px;
        }
        header img { width: 34px; height: 34px; border-radius: 8px; }
        header h1 { font-size: 20px; font-weight: 600; letter-spacing: 0.3px; }
        header .ver {
            margin-left: auto;
            color: var(--muted);
            font-size: 12px;
        }

        /* ── Status chips ── */
        .chips {
            display: flex;
            flex-wrap: wrap;
            gap: 10px;
            margin-bottom: 22px;
        }
        .chip {
            background: var(--card);
            border: 1px solid var(--border-soft);
            border-radius: 10px;
            padding: 10px 16px;
            display: flex;
            flex-direction: column;
            gap: 2px;
            min-width: 110px;
        }
        .chip .num { font-size: 20px; font-weight: 700; line-height: 1.1; }
        .chip .lbl { font-size: 11.5px; color: var(--muted); }
        .chip .num.dot::before {
            content: '';
            display: inline-block;
            width: 9px; height: 9px;
            border-radius: 50%;
            margin-right: 7px;
            vertical-align: 2px;
        }
        .dot-ok::before { background: var(--ok); }
        .dot-warn::before { background: var(--warn); }
        .dot-bad::before { background: var(--bad); }

        /* ── Buttons ── */
        .btn {
            background: var(--card);
            border: 1px solid var(--border);
            color: var(--text);
            border-radius: 8px;
            padding: 8px 14px;
            font-size: 13px;
            cursor: pointer;
            transition: background .15s ease, border-color .15s ease;
        }
        .btn:hover { background: var(--card-hover); border-color: #555; }
        .btn:disabled { opacity: .45; cursor: default; }
        .btn.primary {
            background: #fff;
            color: #000;
            border-color: #fff;
            font-weight: 600;
        }
        .btn.primary:hover { background: #e5e5e5; }
        .btn.sm { padding: 4px 10px; font-size: 12px; }

        .hero {
            display: flex;
            gap: 10px;
            margin-bottom: 24px;
        }
        .hero .btn.primary { padding: 12px 22px; font-size: 14.5px; }

        /* ── Cards ── */
        .card {
            background: var(--card);
            border: 1px solid var(--border-soft);
            border-radius: 14px;
            padding: 18px;
            margin-bottom: 16px;
        }
        .card h2 {
            font-size: 13px;
            font-weight: 600;
            letter-spacing: 1.4px;
            text-transform: uppercase;
            color: var(--muted);
            margin-bottom: 14px;
        }
        .card-head { display: flex; align-items: center; gap: 10px; }
        .card-head h2 { margin-bottom: 0; }
        .card-head .spacer { flex: 1; }

        .search {
            background: #000;
            border: 1px solid var(--border);
            color: var(--text);
            border-radius: 8px;
            padding: 6px 10px;
            font-size: 12.5px;
            width: 190px;
        }
        .search:focus { outline: none; border-color: #666; }

        /* ── List rows ── */
        .row {
            display: flex;
            align-items: center;
            gap: 14px;
            padding: 11px 0;
            border-bottom: 1px solid var(--border-soft);
        }
        .row:last-child { border-bottom: none; padding-bottom: 2px; }
        .row .info { flex: 1; min-width: 0; }
        .row .name { font-size: 14px; font-weight: 600; margin-bottom: 3px; }
        .row .meta { font-size: 12px; color: var(--muted); }
        .empty {
            color: var(--muted);
            font-size: 13px;
            padding: 14px 0 6px;
        }

        /* ── Settings ── */
        .setting {
            display: flex;
            align-items: center;
            gap: 14px;
            padding: 12px 0;
            border-bottom: 1px solid var(--border-soft);
        }
        .setting:last-child { border-bottom: none; }
        .setting .s-info { flex: 1; }
        .setting .s-name { font-size: 14px; margin-bottom: 3px; }
        .setting .s-desc { font-size: 12px; color: var(--muted); }

        .switch { position: relative; width: 42px; height: 24px; flex-shrink: 0; }
        .switch input { opacity: 0; width: 0; height: 0; }
        .switch .slider {
            position: absolute; inset: 0;
            background: #2a2a2a;
            border: 1px solid var(--border);
            border-radius: 24px;
            cursor: pointer;
            transition: background .15s ease;
        }
        .switch .slider::before {
            content: '';
            position: absolute;
            width: 16px; height: 16px;
            left: 3px; top: 3px;
            background: #bbb;
            border-radius: 50%;
            transition: all .15s ease;
        }
        .switch input:checked + .slider { background: #fff; border-color: #fff; }
        .switch input:checked + .slider::before { background: #000; transform: translateX(18px); }

        .folder-line {
            display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
            font-size: 12.5px;
        }
        .folder-line code {
            color: var(--muted);
            font-family: Consolas, monospace;
            font-size: 11.5px;
            word-break: break-all;
        }

        select {
            background: #000;
            border: 1px solid var(--border);
            color: var(--text);
            border-radius: 8px;
            padding: 6px 10px;
            font-size: 12.5px;
            cursor: pointer;
        }

        /* ── Toasts ── */
        #toasts {
            position: fixed;
            bottom: 20px; right: 20px;
            display: flex; flex-direction: column; gap: 8px;
            z-index: 100;
        }
        .toast {
            background: #161616;
            border: 1px solid var(--border);
            border-left: 3px solid var(--ok);
            color: var(--text);
            padding: 11px 16px;
            border-radius: 8px;
            font-size: 13px;
            max-width: 340px;
            animation: pop .18s ease;
        }
        .toast.err { border-left-color: var(--bad); }
        @keyframes pop { from { opacity: 0; transform: translateY(6px); } }

        .footer {
            text-align: center;
            color: #555;
            font-size: 11.5px;
            margin-top: 34px;
        }

        @media (max-width: 620px) {
            .row .meta { display: none; }
            .hero { flex-direction: column; }
            .hero .btn { width: 100%; }
        }
    </style>
</head>
<body>
    <div class="wrap">
        <header>
            <img src="/api/logo" alt="Sager">
            <h1>Sager</h1>
            <span class="ver" id="ver"></span>
        </header>

        <div class="chips">
            <div class="chip"><span class="num" id="stat-games">–</span><span class="lbl">Games found</span></div>
            <div class="chip"><span class="num" id="stat-trophies">–</span><span class="lbl">Trophies unlocked</span></div>
            <div class="chip"><span class="num dot" id="stat-cloud-ok">–</span><span class="lbl">Google Drive</span></div>
        </div>

        <div class="hero">
            <button class="btn primary" id="btn-backup-all" onclick="backupAll()">Back up all games</button>
            <button class="btn" onclick="refresh()">Refresh</button>
        </div>

        <div class="card">
            <div class="card-head">
                <h2>Game saves</h2>
                <span class="spacer"></span>
                <input class="search" id="search" type="text" placeholder="Filter games..." oninput="renderGames()">
            </div>
            <div id="games"><div class="empty">Scanning…</div></div>
        </div>

        <div class="card">
            <div class="card-head">
                <h2>Cloud backups</h2>
                <span class="spacer"></span>
                <button class="btn sm" onclick="openFolder()">Open folder</button>
            </div>
            <div id="backups"><div class="empty">Loading…</div></div>
        </div>

        <div class="card">
            <div class="card-head"><h2>Achievements</h2></div>
            <div id="achievements"><div class="empty">Loading…</div></div>
        </div>

        <div class="card">
            <div class="card-head"><h2>Settings</h2></div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">Back up automatically</div>
                    <div class="s-desc">Quietly saves any game that changed, in the background.</div>
                </div>
                <label class="switch"><input type="checkbox" id="cfg-autobackup" onchange="saveSettings()"><span class="slider"></span></label>
            </div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">How often</div>
                    <div class="s-desc">Interval between automatic backup passes.</div>
                </div>
                <select id="cfg-interval" onchange="saveSettings()">
                    <option value="30">Every 30 minutes</option>
                    <option value="60">Every hour</option>
                    <option value="360">Every 6 hours</option>
                    <option value="1440">Once a day</option>
                </select>
            </div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">Achievement sound</div>
                    <div class="s-desc">Plays the Steam unlock chime.</div>
                </div>
                <label class="switch"><input type="checkbox" id="cfg-sound" onchange="saveSettings()"><span class="slider"></span></label>
            </div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">Windows alerts</div>
                    <div class="s-desc">Shows the in-game toast and tray bubble.</div>
                </div>
                <label class="switch"><input type="checkbox" id="cfg-toasts" onchange="saveSettings()"><span class="slider"></span></label>
            </div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">Start with Windows</div>
                    <div class="s-desc">Runs silently in the tray on boot.</div>
                </div>
                <label class="switch"><input type="checkbox" id="cfg-startup" onchange="setStartup(this.checked)"><span class="slider"></span></label>
            </div>

            <div class="setting">
                <div class="s-info">
                    <div class="s-name">Backup folder</div>
                    <div class="s-desc">
                        <span class="folder-line">
                            <code id="cfg-folder"></code>
                            <button class="btn sm" onclick="openFolder()">Open</button>
                            <button class="btn sm" id="btn-drive" onclick="startDrive()" style="display:none">Start Google Drive</button>
                        </span>
                    </div>
                </div>
            </div>
        </div>

        <div class="footer">
            Sager keeps your saves safe. They live in the folder above, which syncs with Google Drive when it is running.
        </div>
    </div>

    <div id="toasts"></div>

    <script>
        let GAMES = [];
        let BACKUPS = {};
        let BUSY = false;

        async function api(url, body) {
            const opts = body ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) } : {};
            const res = await fetch(url, opts);
            return res.json();
        }

        function toast(msg, err) {
            const t = document.createElement('div');
            t.className = 'toast' + (err ? ' err' : '');
            t.textContent = msg;
            document.getElementById('toasts').appendChild(t);
            setTimeout(() => t.remove(), 5000);
        }

        function fmtBytes(n) {
            if (!n) return '0 B';
            const u = ['B', 'KB', 'MB', 'GB'];
            let i = 0;
            while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
            return (i ? n.toFixed(1) : n) + ' ' + u[i];
        }

        function fmtDate(ts) {
            if (!ts) return '—';
            return new Date(ts * 1000).toLocaleString();
        }

        // ── Data loading ──

        async function refresh() {
            const [status, games, backups, achs] = await Promise.all([
                api('/api/status'),
                api('/api/games'),
                api('/api/backups'),
                api('/api/achievements')
            ]).catch(() => [null, null, null, null]);

            if (!status) { toast('Lost contact with Sager — is the tray app running?', true); return; }

            document.getElementById('ver').textContent = 'v' + status.version;
            document.getElementById('stat-games').textContent = status.games_count;
            document.getElementById('stat-trophies').textContent = status.achievements_count;

            const cloud = document.getElementById('stat-cloud-ok');
            const ready = status.backup_dir_accessible;
            cloud.classList.remove('dot-ok', 'dot-warn', 'dot-bad');
            cloud.classList.add(ready ? 'dot-ok' : 'dot-warn');
            cloud.textContent = ready ? 'Syncing' : 'Not ready';

            document.getElementById('btn-drive').style.display =
                (!ready && status.google_drive_installed) ? '' : 'none';
            document.getElementById('cfg-folder').textContent = status.backup_dir;

            document.getElementById('cfg-autobackup').checked = status.auto_backup_enabled;
            const interval = document.getElementById('cfg-interval');
            const minutes = String(status.auto_backup_interval_minutes);
            const match = Array.from(interval.options).some(o => o.value === minutes);
            if (!match) {
                const opt = document.createElement('option');
                opt.value = minutes;
                opt.textContent = 'Every ' + status.auto_backup_interval_minutes + ' minutes';
                interval.appendChild(opt);
            }
            interval.value = minutes;
            document.getElementById('cfg-sound').checked = status.sound_enabled;
            document.getElementById('cfg-toasts').checked = status.notifications_enabled;
            document.getElementById('cfg-startup').checked = status.startup_enabled;

            GAMES = games || [];
            BACKUPS = backups || {};
            renderGames();
            renderBackups(BACKUPS);
            renderAchievements(achs || []);
        }

        function renderGames() {
            const q = document.getElementById('search').value.toLowerCase();
            const el = document.getElementById('games');
            const list = GAMES.filter(g => g.name.toLowerCase().includes(q));

            if (!GAMES.length) {
                el.innerHTML = '<div class="empty">No game saves found yet. Open a game you own, save your progress, then press Refresh.</div>';
                return;
            }
            if (!list.length) {
                el.innerHTML = '<div class="empty">No matches.</div>';
                return;
            }

            el.innerHTML = list.map(g => {
                const when = g.last_backup
                    ? 'Last backup ' + g.last_backup
                    : g.backups_count ? 'Backup pending (changed since last pass)' : 'Never backed up';
                return '<div class="row">' +
                    '<div class="info">' +
                    '<div class="name">' + esc(g.name) + '</div>' +
                    '<div class="meta">' + esc(g.save_category) + ' &middot; ' + g.files_count + ' files &middot; ' + fmtBytes(g.total_size_bytes) + ' &middot; ' + when + '</div>' +
                    '</div>' +
                    '<button class="btn sm" onclick="backupGame(\'' + enc(g.source_path) + '\', \'' + enc(g.name) + '\')">Back up</button>' +
                    '</div>';
            }).join('');
        }

        function renderBackups(data) {
            BACKUPS = data;
            const el = document.getElementById('backups');
            const groups = Object.entries(data);

            if (!groups.length) {
                el.innerHTML = '<div class="empty">Nothing in the cloud yet. Press Back up all games to make your first snapshot.</div>';
                return;
            }

            el.innerHTML = groups.map(([game, arcs]) => {
                const newest = arcs[0];
                const total = arcs.reduce((s, a) => s + (a.size_bytes || 0), 0);
                const when = newest && newest.manifest ? newest.manifest.backup_date_str : '—';
                return '<div class="row">' +
                    '<div class="info">' +
                    '<div class="name">' + esc(game) + '</div>' +
                    '<div class="meta">' + arcs.length + ' snapshots &middot; newest ' + when + ' &middot; ' + fmtBytes(total) + '</div>' +
                    '</div>' +
                    '<button class="btn sm" onclick="askRestore(\'' + enc(game) + '\')">Restore</button>' +
                    '</div>';
            }).join('');
        }

        function renderAchievements(list) {
            const el = document.getElementById('achievements');

            if (!list.length) {
                el.innerHTML = '<div class="empty">No trophies yet. They appear here (and pop up in-game) the moment you unlock them.</div>';
                return;
            }

            el.innerHTML = list.slice(0, 25).map(a =>
                '<div class="row">' +
                '<div class="info">' +
                '<div class="name">' + esc(a.title) + '</div>' +
                '<div class="meta">' + esc(a.game_name) + ' &middot; ' + fmtDate(a.unlock_time) + '</div>' +
                '</div>' +
                '</div>'
            ).join('') +
            (list.length > 25 ? '<div class="empty">…and ' + (list.length - 25) + ' more</div>' : '');
        }

        // ── Actions ──

        function enc(s) { return encodeURIComponent(s); }
        function esc(s) {
            return String(s || '').replace(/[&<>"']/g, c =>
                ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
        }

        async function runAction(fn, busyMsg) {
            if (BUSY) { toast('Hang on, still working on the last request…'); return; }
            BUSY = true;
            toast(busyMsg);
            try {
                const r = await fn();
                toast(r.message || 'Done');
            } catch (e) {
                toast('Something went wrong: ' + e, true);
            }
            BUSY = false;
            await refresh();
        }

        function backupAll() {
            runAction(() => api('/api/backup', { all: true }), 'Backing up every game…');
        }

        function backupGame(path, name) {
            runAction(() => api('/api/backup', { source_path: path, game_name: name }), 'Backing up ' + name + '…');
        }

        function askRestore(game) {
            const arcs = BACKUPS[game] || [];
            if (!arcs.length) return;
            const newest = arcs[0];
            if (!window.confirm('Replace the saves of "' + game + '" on this PC with the backup from ' +
                (newest.manifest ? newest.manifest.backup_date_str : 'the cloud') + '?\n\nYour current saves are kept as a safety copy first.')) return;
            runAction(() => api('/api/restore', { zip_path: newest.full_path }), 'Restoring ' + game + '…');
        }

        function openFolder() { api('/api/open-folder', {}); }
        function startDrive() {
            runAction(() => api('/api/launch-drive', {}), 'Starting Google Drive…');
        }

        function saveSettings() {
            const body = {
                auto_backup_enabled: document.getElementById('cfg-autobackup').checked,
                auto_backup_interval_minutes: parseInt(document.getElementById('cfg-interval').value, 10),
                sound_enabled: document.getElementById('cfg-sound').checked,
                notifications_enabled: document.getElementById('cfg-toasts').checked
            };
            api('/api/config', body).then(() => toast('Settings saved'));
        }

        function setStartup(enabled) {
            api('/api/startup', { enabled }).then(r => {
                toast(r.message || (r.enabled ? 'Sager will start with Windows' : 'Sager will not start with Windows'));
            });
        }

        refresh();
    </script>
</body>
</html>
"#
}
