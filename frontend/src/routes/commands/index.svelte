<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    import { Button, Space } from "@svelteuidev/core"
    import { PlusCircled, Trash, Reload, Cross2, Play, Pencil1, LightningBolt } from "radix-icons-svelte"

    import HDivider from "@/components/elements/HDivider.svelte"
    import Footer from "@/components/Footer.svelte"

    // ── Tab ──────────────────────────────────────────────────────────────────────

    let activeTab: "commands" | "scripts" = "commands"

    // ════════════════════════════════════════════════════════════════════════════
    //  COMMANDS
    // ════════════════════════════════════════════════════════════════════════════

    interface JCommand {
        id: string; type: string; description: string
        exe_path: string; exe_args: string[]; cli_cmd: string; cli_args: string[]
        phrases: Record<string, string[]>; patterns: string[]
        response_sound: string
    }
    interface CommandPack { pack_name: string; commands: JCommand[] }

    let packs: CommandPack[] = []
    let availableSoundFiles: string[] = []
    let cmdLoading = true
    let cmdError = ""
    let cmdSuccess = ""
    let saving = false
    let showCmdModal = false
    let deletePackTarget = ""
    let deleteCmdTarget: { pack: string; id: string } | null = null

    const CHROME = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"

    type CmdModalMode = "add" | "edit"
    let cmdModalMode: CmdModalMode = "add"
    let editingPack = ""
    let editingOldId = ""
    let cmdIdTouched = false

    interface CmdForm {
        packTarget: string; newPackName: string; id: string; cmd_type: string
        description: string; exe_path: string; exe_args: string; url: string
        cli_cmd: string; cli_args: string; phrases_ru: string; phrases_en: string
        patterns: string; response_sound: string; formError: string
    }
    let cmdForm: CmdForm = emptyCmdForm()

    function emptyCmdForm(): CmdForm {
        return {
            packTarget: "__new__", newPackName: "", id: "", cmd_type: "exe",
            description: "", exe_path: "", exe_args: "", url: "", cli_cmd: "",
            cli_args: "", phrases_ru: "", phrases_en: "", patterns: "",
            response_sound: "", formError: "",
        }
    }

    function splitTrim(s: string): string[] {
        return s.split(",").map(x => x.trim()).filter(Boolean)
    }

    $: if (cmdModalMode === "add" && cmdForm.packTarget === "__new__" && cmdForm.newPackName && !cmdIdTouched) {
        cmdForm.id = cmdForm.newPackName.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "")
    }

    async function loadPacks() {
        cmdLoading = true; cmdError = ""
        try { packs = await invoke<CommandPack[]>("list_command_packs") }
        catch (e) { cmdError = String(e) }
        cmdLoading = false
    }

    async function saveCmd() {
        cmdForm.formError = ""
        const effectivePack = cmdModalMode === "edit"
            ? editingPack
            : (cmdForm.packTarget === "__new__" ? cmdForm.newPackName.trim() : cmdForm.packTarget)

        if (!effectivePack)           { cmdForm.formError = "Pack name is required"; return }
        if (!cmdForm.id.trim())       { cmdForm.formError = "Command ID is required"; return }
        if (!cmdForm.phrases_ru.trim() && !cmdForm.phrases_en.trim()) {
            cmdForm.formError = "At least one phrase (RU or EN) is required"; return
        }

        let rustType: string, exePath: string, exeArgs: string[]
        if (cmdForm.cmd_type === "chrome") {
            rustType = "exe"; exePath = CHROME
            exeArgs = cmdForm.url.trim() ? [cmdForm.url.trim()] : []
        } else {
            rustType = cmdForm.cmd_type; exePath = cmdForm.exe_path.trim()
            exeArgs = splitTrim(cmdForm.exe_args)
        }

        const payload = {
            type: rustType, id: cmdForm.id.trim(), description: cmdForm.description.trim(),
            phrases_ru: splitTrim(cmdForm.phrases_ru), phrases_en: splitTrim(cmdForm.phrases_en),
            exe_path: exePath, exe_args: exeArgs, cli_cmd: cmdForm.cli_cmd.trim(),
            cli_args: splitTrim(cmdForm.cli_args), patterns: splitTrim(cmdForm.patterns),
            sounds_ru: ["ok1", "ok2", "ok3"],
            response_sound: cmdForm.response_sound,
        }

        saving = true
        try {
            if (cmdModalMode === "edit") {
                await invoke("update_command", { packName: editingPack, oldId: editingOldId, command: payload })
                flashCmd(`Command "${cmdForm.id}" updated`)
            } else if (cmdForm.packTarget === "__new__") {
                await invoke("create_command_pack", { packName: effectivePack, command: payload })
                flashCmd(`Pack "${effectivePack}" created`)
            } else {
                await invoke("append_command_to_pack", { packName: effectivePack, command: payload })
                flashCmd(`Command "${cmdForm.id}" added`)
            }
            // Hot-reload: push changes to running jarvis-app without restart.
            // Silently ignored if jarvis-app is not running.
            invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
            closeCmdModal(); await loadPacks()
        } catch (e) { cmdForm.formError = String(e) }
        saving = false
    }

    async function deletePack(name: string) {
        if (deletePackTarget !== name) {
            deletePackTarget = name; setTimeout(() => { deletePackTarget = "" }, 3000); return
        }
        deletePackTarget = ""
        try {
            await invoke("delete_command_pack", { packName: name })
            invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
            await loadPacks()
        }
        catch (e) { cmdError = String(e) }
    }

    async function deleteCmd(pack: string, id: string) {
        if (!deleteCmdTarget || deleteCmdTarget.pack !== pack || deleteCmdTarget.id !== id) {
            deleteCmdTarget = { pack, id }; setTimeout(() => { deleteCmdTarget = null }, 3000); return
        }
        deleteCmdTarget = null
        try {
            await invoke("delete_command", { packName: pack, commandId: id })
            invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
            await loadPacks()
        }
        catch (e) { cmdError = String(e) }
    }

    function openAddCmdModal() {
        cmdModalMode = "add"; cmdForm = emptyCmdForm()
        cmdForm.packTarget = packs.length > 0 ? packs[0].pack_name : "__new__"
        cmdIdTouched = false; showCmdModal = true
    }

    function openEditCmdModal(packName: string, cmd: JCommand) {
        cmdModalMode = "edit"; editingPack = packName; editingOldId = cmd.id
        cmdIdTouched = true
        let uiType = cmd.type, url = ""
        if (cmd.type === "exe" && cmd.exe_path?.toLowerCase().includes("chrome")) {
            uiType = "chrome"; url = cmd.exe_args?.[0] ?? ""
        }
        cmdForm = {
            packTarget: packName, newPackName: "", id: cmd.id, cmd_type: uiType,
            description: cmd.description ?? "", exe_path: uiType === "exe" ? cmd.exe_path : "",
            exe_args: cmd.exe_args?.join(", ") ?? "", url,
            cli_cmd: cmd.cli_cmd ?? "", cli_args: cmd.cli_args?.join(", ") ?? "",
            phrases_ru: (cmd.phrases?.ru ?? []).join(", "), phrases_en: (cmd.phrases?.en ?? []).join(", "),
            patterns: cmd.patterns?.join(", ") ?? "",
            response_sound: cmd.response_sound ?? "", formError: "",
        }
        showCmdModal = true
    }

    function closeCmdModal() {
        showCmdModal = false; cmdIdTouched = false; cmdForm = emptyCmdForm()
    }

    function flashCmd(msg: string) { cmdSuccess = msg; setTimeout(() => { cmdSuccess = "" }, 4000) }

    function typeBadge(cmd: JCommand) {
        if (cmd.type === "exe" && cmd.exe_path?.toLowerCase().includes("chrome")) return "Chrome"
        return cmd.type.toUpperCase()
    }
    function typeColor(cmd: JCommand) {
        const t = cmd.type
        if (t === "exe") return "#52fefe"; if (t === "cli") return "#8AC832"
        if (t === "url") return "#a78bfa"; if (t === "lua") return "#fb923c"
        if (t === "voice") return "#94a3b8"; return "#64748b"
    }
    function displayPath(cmd: JCommand) {
        if (cmd.exe_path) return cmd.exe_path + (cmd.exe_args?.length ? ` ${cmd.exe_args.join(" ")}` : "")
        if (cmd.cli_cmd)  return `${cmd.cli_cmd} ${(cmd.cli_args||[]).join(" ")}`
        return "—"
    }
    function displayPhrases(cmd: JCommand) {
        const ru = cmd.phrases?.ru ?? [], en = cmd.phrases?.en ?? []
        const all = [...ru, ...en].slice(0, 3)
        return all.length ? all.join(", ") + (ru.length + en.length > 3 ? "…" : "") : "—"
    }
    function isCmdDeletePending(pack: string, id: string) {
        return deleteCmdTarget?.pack === pack && deleteCmdTarget?.id === id
    }

    // ════════════════════════════════════════════════════════════════════════════
    //  SCRIPTS
    // ════════════════════════════════════════════════════════════════════════════

    interface ScriptStep {
        step_type: "command_ref" | "delay" | "custom" | "spotify"
        label: string; pack: string; command_id: string
        delay_ms: number; cli_cmd: string; cli_args: string[]
        spotify_action: string; spotify_track_id: string
    }
    interface Script {
        id: string; name: string; description: string
        mode: "sequential" | "parallel"
        steps: ScriptStep[]
        phrases_ru: string[]; phrases_en: string[]; patterns: string[]; sounds_ru: string[]
        response_sound: string
    }

    let scripts: Script[] = []
    let scrLoading = true
    let scrError = ""
    let scrSuccess = ""
    let showScrModal = false
    let scrSaving = false
    let runningId = ""
    let deleteScrTarget = ""

    type ScrModalMode = "add" | "edit"
    let scrModalMode: ScrModalMode = "add"
    let scrIdTouched = false

    interface ScrForm {
        id: string; name: string; description: string
        mode: "sequential" | "parallel"
        steps: FormStep[]
        phrases_ru: string; phrases_en: string; patterns: string
        response_sound: string; formError: string
    }
    interface FormStep {
        step_type: "command_ref" | "delay" | "custom" | "spotify"
        label: string; pack: string; command_id: string
        delay_ms: string; cli_cmd: string; cli_args: string
        spotify_action: string; spotify_track_id: string
    }

    let scrForm: ScrForm = emptyScrForm()

    function emptyScrForm(): ScrForm {
        return { id: "", name: "", description: "", mode: "sequential",
                 steps: [], phrases_ru: "", phrases_en: "", patterns: "",
                 response_sound: "", formError: "" }
    }
    function emptyStep(): FormStep {
        return { step_type: "command_ref", label: "",
                 pack: packs[0]?.pack_name ?? "", command_id: "",
                 delay_ms: "500", cli_cmd: "", cli_args: "",
                 spotify_action: "play_track", spotify_track_id: "" }
    }

    $: if (scrForm.name && !scrIdTouched) {
        scrForm.id = scrForm.name.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "")
    }

    async function loadScripts() {
        scrLoading = true; scrError = ""
        try { scripts = await invoke<Script[]>("list_scripts") }
        catch (e) { scrError = String(e) }
        scrLoading = false
    }

    async function runScript(id: string) {
        runningId = id
        try { await invoke("run_script", { scriptId: id }); flashScr(`Script "${id}" launched`) }
        catch (e) { scrError = String(e) }
        runningId = ""
    }

    async function deleteScript(id: string) {
        if (deleteScrTarget !== id) {
            deleteScrTarget = id; setTimeout(() => { deleteScrTarget = "" }, 3000); return
        }
        deleteScrTarget = ""
        try { await invoke("delete_script", { scriptId: id }); await loadScripts() }
        catch (e) { scrError = String(e) }
    }

    async function saveScript() {
        scrForm.formError = ""
        if (!scrForm.id.trim())   { scrForm.formError = "Script ID is required"; return }
        if (!scrForm.name.trim()) { scrForm.formError = "Script name is required"; return }
        if (scrForm.steps.length === 0) { scrForm.formError = "Add at least one step"; return }

        const steps: ScriptStep[] = scrForm.steps.map(s => ({
            step_type: s.step_type, label: s.label.trim(),
            pack: s.pack, command_id: s.command_id,
            delay_ms: parseInt(s.delay_ms) || 500,
            cli_cmd: s.cli_cmd.trim(),
            cli_args: s.cli_args.split(",").map(x => x.trim()).filter(Boolean),
            spotify_action: s.spotify_action,
            spotify_track_id: s.spotify_track_id.trim(),
        }))

        const payload: Script = {
            id: scrForm.id.trim(), name: scrForm.name.trim(), description: scrForm.description.trim(),
            mode: scrForm.mode, steps,
            phrases_ru: scrForm.phrases_ru.split(",").map(x => x.trim()).filter(Boolean),
            phrases_en: scrForm.phrases_en.split(",").map(x => x.trim()).filter(Boolean),
            patterns: scrForm.patterns.split(",").map(x => x.trim()).filter(Boolean),
            sounds_ru: ["ok1", "ok2", "ok3"],
            response_sound: scrForm.response_sound,
        }

        scrSaving = true
        try {
            await invoke("save_script", { script: payload })
            flashScr(scrModalMode === "edit" ? `"${payload.name}" updated` : `"${payload.name}" created`)
            closeScrModal(); await loadScripts()
        } catch (e) { scrForm.formError = String(e) }
        scrSaving = false
    }

    function openAddScrModal() {
        scrModalMode = "add"; scrForm = emptyScrForm(); scrIdTouched = false; showScrModal = true
    }
    function openEditScrModal(s: Script) {
        scrModalMode = "edit"; scrIdTouched = true
        scrForm = {
            id: s.id, name: s.name, description: s.description, mode: s.mode,
            steps: s.steps.map(st => ({
                step_type: st.step_type, label: st.label, pack: st.pack,
                command_id: st.command_id, delay_ms: String(st.delay_ms),
                cli_cmd: st.cli_cmd, cli_args: st.cli_args.join(", "),
                spotify_action: st.spotify_action || "play_track",
                spotify_track_id: st.spotify_track_id || "",
            })),
            phrases_ru: s.phrases_ru.join(", "), phrases_en: s.phrases_en.join(", "),
            patterns: s.patterns.join(", "),
            response_sound: s.response_sound || "", formError: "",
        }
        showScrModal = true
    }
    function closeScrModal() { showScrModal = false; scrForm = emptyScrForm(); scrIdTouched = false }

    function addStep()            { scrForm.steps = [...scrForm.steps, emptyStep()] }
    function removeStep(i: number){ scrForm.steps = scrForm.steps.filter((_, idx) => idx !== i) }
    function moveStep(i: number, dir: -1 | 1) {
        const j = i + dir
        if (j < 0 || j >= scrForm.steps.length) return
        const arr = [...scrForm.steps]; [arr[i], arr[j]] = [arr[j], arr[i]]; scrForm.steps = arr
    }

    function flashScr(msg: string) { scrSuccess = msg; setTimeout(() => { scrSuccess = "" }, 4000) }

    function modeColor(mode: string) { return mode === "parallel" ? "#a78bfa" : "#52fefe" }
    function modeLabel(mode: string) { return mode === "parallel" ? "Parallel" : "Sequential" }
    function stepSummary(s: ScriptStep) {
        if (s.step_type === "command_ref") return `${s.pack} › ${s.command_id}`
        if (s.step_type === "delay")       return `Wait ${s.delay_ms} ms`
        if (s.step_type === "spotify") {
            if (s.spotify_action === "pause") return "Spotify: Pause"
            if (s.spotify_action === "next")  return "Spotify: Next"
            return `Spotify: ${s.spotify_track_id || "play track"}`
        }
        return s.cli_cmd || "Shell"
    }
    function stepIcon(t: string) {
        if (t === "command_ref") return "⚡"; if (t === "delay") return "⏱"
        if (t === "spotify") return "🎵"; return ">"
    }
    function packCmds(packName: string) {
        return packs.find(p => p.pack_name === packName)?.commands ?? []
    }

    // ── Init ─────────────────────────────────────────────────────────────────────
    async function loadAll() {
        await loadPacks()
        await loadScripts()
        try { availableSoundFiles = await invoke<string[]>("list_sound_files") } catch {}
    }

    onMount(loadAll)
</script>

<!-- ── Tab switcher ──────────────────────────────────────────────────────────── -->
<Space h="xl" />

<div class="tab-bar">
    <button class="tab" class:active={activeTab === "commands"} on:click={() => activeTab = "commands"}>
        Commands
        {#if !cmdLoading}
            <span class="tab-count">{packs.reduce((s,p) => s + p.commands.length, 0)}</span>
        {/if}
    </button>
    <button class="tab" class:active={activeTab === "scripts"} on:click={() => activeTab = "scripts"}>
        <LightningBolt size={13} />
        Scripts
        {#if !scrLoading}
            <span class="tab-count">{scripts.length}</span>
        {/if}
    </button>
</div>

<!-- ════════════════════════════════════════════════════════════════════════════ -->
{#if activeTab === "commands"}
<!-- ════════════════════════════════════════════════════════════════════════════ -->

{#if cmdSuccess}<div class="toast success">{cmdSuccess}</div>{/if}

<div class="page-header">
    <p class="page-sub">
        {cmdLoading ? "Loading…" : `${packs.reduce((s,p) => s+p.commands.length,0)} commands in ${packs.length} packs`}
    </p>
    <div class="header-actions">
        <button class="icon-btn" title="Refresh" on:click={loadPacks}><Reload size={16} /></button>
        <Button color="lime" radius="md" size="sm" uppercase ripple on:click={openAddCmdModal}>
            <PlusCircled size={14} />&nbsp;Add Command
        </Button>
    </div>
</div>

{#if cmdError}<div class="toast error">{cmdError}</div>{/if}
<Space h="md" />

{#if cmdLoading}
    <div class="empty-state">Loading command packs…</div>
{:else if packs.length === 0}
    <div class="empty-state">
        <p>No commands yet.</p>
        <Button color="lime" radius="md" size="sm" uppercase on:click={openAddCmdModal}>Add your first command</Button>
    </div>
{:else}
    {#each packs as pack}
        <div class="pack-section">
            <div class="pack-header">
                <span class="pack-name">{pack.pack_name}</span>
                <div class="pack-header-right">
                    <button class="icon-btn-sm" title="Add command to this pack"
                        on:click={() => { cmdModalMode="add"; cmdForm=emptyCmdForm(); cmdForm.packTarget=pack.pack_name; cmdIdTouched=false; showCmdModal=true }}>
                        <PlusCircled size={13} />
                    </button>
                    <button class="delete-pack-btn" class:confirm={deletePackTarget===pack.pack_name}
                        title={deletePackTarget===pack.pack_name ? "Click again to confirm" : "Delete pack"}
                        on:click={() => deletePack(pack.pack_name)}>
                        <Trash size={13} />
                        {deletePackTarget === pack.pack_name ? "Confirm?" : "Delete pack"}
                    </button>
                </div>
            </div>

            <table class="cmd-table">
                <thead><tr>
                    <th>ID</th><th>Type</th><th>Phrases</th><th>Path / URL</th>
                    <th class="th-actions"></th>
                </tr></thead>
                <tbody>
                    {#each pack.commands as cmd}
                        <tr>
                            <td class="cell-id">{cmd.id}</td>
                            <td>
                                <span class="badge" style="color:{typeColor(cmd)};border-color:{typeColor(cmd)}22">
                                    {typeBadge(cmd)}
                                </span>
                            </td>
                            <td class="cell-phrases">{displayPhrases(cmd)}</td>
                            <td class="cell-path">{displayPath(cmd)}</td>
                            <td class="cell-actions">
                                <button class="row-btn edit-btn" title="Edit" on:click={() => openEditCmdModal(pack.pack_name, cmd)}>
                                    <Pencil1 size={13} />
                                </button>
                                <button class="row-btn trash-btn"
                                    class:confirm={isCmdDeletePending(pack.pack_name, cmd.id)}
                                    title={isCmdDeletePending(pack.pack_name, cmd.id) ? "Confirm?" : "Delete command"}
                                    on:click={() => deleteCmd(pack.pack_name, cmd.id)}>
                                    <Trash size={13} />
                                </button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/each}
{/if}

<!-- ════════════════════════════════════════════════════════════════════════════ -->
{:else}
<!-- ════════════════════════════════════════════════════════════════════════════ -->

{#if scrSuccess}<div class="toast success">{scrSuccess}</div>{/if}

<div class="page-header">
    <p class="page-sub">
        {scrLoading ? "Loading…" : `${scripts.length} automation script${scripts.length !== 1 ? "s" : ""}`}
    </p>
    <div class="header-actions">
        <button class="icon-btn" title="Refresh" on:click={loadScripts}><Reload size={16} /></button>
        <Button color="lime" radius="md" size="sm" uppercase ripple on:click={openAddScrModal}>
            <PlusCircled size={14} />&nbsp;Add Script
        </Button>
    </div>
</div>

{#if scrError}<div class="toast error">{scrError}</div>{/if}
<Space h="md" />

{#if scrLoading}
    <div class="empty-state">Loading scripts…</div>
{:else if scripts.length === 0}
    <div class="empty-state">
        <p>No scripts yet.</p>
        <p class="empty-hint">A script is a sequence of commands — run them one after another (Sequential) or all at once (Parallel).</p>
        <Button color="lime" radius="md" size="sm" uppercase on:click={openAddScrModal}>Create your first script</Button>
    </div>
{:else}
    <div class="script-grid">
        {#each scripts as s}
            <div class="script-card">
                <div class="card-header">
                    <div class="card-title-row">
                        <span class="card-name" title={s.name}>{s.name}</span>
                        <span class="mode-badge" style="color:{modeColor(s.mode)};border-color:{modeColor(s.mode)}33">
                            {modeLabel(s.mode)}
                        </span>
                    </div>
                    {#if s.description}
                        <p class="card-desc" title={s.description}>{s.description}</p>
                    {/if}
                </div>
                <div class="steps-preview">
                    {#each s.steps.slice(0,4) as step}
                        <div class="step-chip">
                            <span class="step-icon">{stepIcon(step.step_type)}</span>
                            <span class="step-text" title={stepSummary(step)}>{stepSummary(step)}</span>
                        </div>
                    {/each}
                    {#if s.steps.length > 4}<div class="step-chip more">+{s.steps.length - 4} more</div>{/if}
                    {#if s.steps.length === 0}<span class="no-steps">No steps</span>{/if}
                </div>
                <div class="card-footer">
                    <span class="step-count">{s.steps.length} step{s.steps.length !== 1 ? "s" : ""}</span>
                    <div class="card-actions">
                        <button class="action-btn run-btn" disabled={runningId === s.id}
                            title="Run script" on:click={() => runScript(s.id)}>
                            <Play size={13} />
                            {runningId === s.id ? "Running…" : "Run"}
                        </button>
                        <button class="action-btn edit-btn-card" title="Edit" on:click={() => openEditScrModal(s)}>
                            <Pencil1 size={13} />
                        </button>
                        <button class="action-btn trash-btn-card"
                            class:confirm={deleteScrTarget === s.id}
                            title={deleteScrTarget === s.id ? "Click again to confirm" : "Delete script"}
                            on:click={() => deleteScript(s.id)}>
                            <Trash size={13} />
                            {deleteScrTarget === s.id ? "Sure?" : ""}
                        </button>
                    </div>
                </div>
            </div>
        {/each}
    </div>
{/if}

{/if}
<!-- ── End tabs ────────────────────────────────────────────────────────────── -->

<HDivider />
<Footer />

<!-- ═══════════════════════════════════════════════════════════════════════════
     COMMANDS MODAL
════════════════════════════════════════════════════════════════════════════ -->
{#if showCmdModal}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click|self={closeCmdModal}>
        <div class="modal">
            <div class="modal-header">
                <h3>{cmdModalMode === "edit" ? "Edit Command" : "Add Command"}</h3>
                <button class="close-btn" on:click={closeCmdModal}><Cross2 size={16} /></button>
            </div>
            <div class="modal-body">
                {#if cmdModalMode === "add"}
                    <div class="field">
                        <label>Pack <span class="hint">(select existing or create new)</span></label>
                        <select bind:value={cmdForm.packTarget} class="select-input">
                            <option value="__new__">— Create new pack —</option>
                            {#each packs as p}<option value={p.pack_name}>{p.pack_name}</option>{/each}
                        </select>
                    </div>
                    {#if cmdForm.packTarget === "__new__"}
                        <div class="field">
                            <label>New pack name</label>
                            <input bind:value={cmdForm.newPackName} placeholder="my_game" />
                        </div>
                    {/if}
                {:else}
                    <div class="field">
                        <label>Pack</label>
                        <div class="readonly-pack">{editingPack}</div>
                    </div>
                {/if}

                <div class="field">
                    <label>Command ID <span class="hint">(unique{cmdModalMode === "add" ? ", auto-filled" : ""})</span></label>
                    <input bind:value={cmdForm.id} on:input={() => cmdIdTouched = true} placeholder="launch_my_game" />
                </div>

                <div class="field">
                    <label>Type</label>
                    <div class="type-tabs">
                        {#each [{v:"exe",label:"EXE App"},{v:"chrome",label:"Chrome / URL"},{v:"cli",label:"CLI / PowerShell"},{v:"voice",label:"Voice only"}] as tab}
                            <button class="type-tab" class:active={cmdForm.cmd_type===tab.v} on:click={() => cmdForm.cmd_type=tab.v}>{tab.label}</button>
                        {/each}
                    </div>
                </div>

                {#if cmdForm.cmd_type === "exe"}
                    <div class="field"><label>EXE path</label><input bind:value={cmdForm.exe_path} placeholder="C:\Games\game.exe" /></div>
                    <div class="field"><label>Arguments <span class="hint">(comma-separated)</span></label><input bind:value={cmdForm.exe_args} placeholder="-fullscreen" /></div>
                {:else if cmdForm.cmd_type === "chrome"}
                    <div class="field"><label>URL</label><input bind:value={cmdForm.url} placeholder="https://youtube.com" /></div>
                    <div class="chrome-hint">Uses: <code>{CHROME}</code></div>
                {:else if cmdForm.cmd_type === "cli"}
                    <div class="field"><label>Command</label><input bind:value={cmdForm.cli_cmd} placeholder="powershell" /></div>
                    <div class="field"><label>Arguments <span class="hint">(comma-separated)</span></label><input bind:value={cmdForm.cli_args} placeholder="-NoProfile, -Command, echo hi" /></div>
                {/if}

                <div class="field-row">
                    <div class="field">
                        <label>Phrases RU <span class="hint">(comma-sep)</span></label>
                        <textarea bind:value={cmdForm.phrases_ru} rows="2" placeholder="запусти игру, игра"></textarea>
                    </div>
                    <div class="field">
                        <label>Phrases EN</label>
                        <textarea bind:value={cmdForm.phrases_en} rows="2" placeholder="launch game"></textarea>
                    </div>
                </div>
                <div class="field"><label>Regex <span class="hint">(optional)</span></label><input bind:value={cmdForm.patterns} placeholder="игра|game" /></div>
                <div class="field"><label>Description <span class="hint">(optional)</span></label><input bind:value={cmdForm.description} placeholder="Launches My Game" /></div>
                <div class="field">
                    <label>Response Sound <span class="hint">(optional — overrides voice pack)</span></label>
                    <select bind:value={cmdForm.response_sound} class="select-input">
                        <option value="">— random from voice pack —</option>
                        {#each availableSoundFiles as f}
                            <option value={f}>{f.split("/").pop()}</option>
                        {/each}
                    </select>
                </div>
                {#if cmdForm.formError}<div class="form-error">{cmdForm.formError}</div>{/if}
            </div>
            <div class="modal-footer">
                <Button color="gray" radius="md" size="sm" uppercase on:click={closeCmdModal}>Cancel</Button>
                <Button color="lime" radius="md" size="sm" uppercase ripple disabled={saving} on:click={saveCmd}>
                    {saving ? "Saving…" : (cmdModalMode === "edit" ? "Save Changes" : "Add Command")}
                </Button>
            </div>
        </div>
    </div>
{/if}

<!-- ═══════════════════════════════════════════════════════════════════════════
     SCRIPTS MODAL
════════════════════════════════════════════════════════════════════════════ -->
{#if showScrModal}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click|self={closeScrModal}>
        <div class="modal">
            <div class="modal-header">
                <h3>
                    <LightningBolt size={15} style="margin-right:6px;opacity:.7" />
                    {scrModalMode === "edit" ? "Edit Script" : "New Script"}
                </h3>
                <button class="close-btn" on:click={closeScrModal}><Cross2 size={16} /></button>
            </div>
            <div class="modal-body">
                <div class="field-row">
                    <div class="field">
                        <label>Script Name</label>
                        <input bind:value={scrForm.name} placeholder="Morning Routine" />
                    </div>
                    <div class="field">
                        <label>ID <span class="hint">(auto)</span></label>
                        <input bind:value={scrForm.id} on:input={() => scrIdTouched=true} placeholder="morning_routine" />
                    </div>
                </div>
                <div class="field">
                    <label>Description <span class="hint">(optional)</span></label>
                    <input bind:value={scrForm.description} placeholder="What this script does…" />
                </div>

                <div class="field">
                    <label>Execution Mode</label>
                    <div class="mode-tabs">
                        <button class="mode-tab" class:active={scrForm.mode==="sequential"} on:click={() => scrForm.mode="sequential"}>
                            ▶▶ Sequential <span class="mode-hint">one after another</span>
                        </button>
                        <button class="mode-tab" class:active={scrForm.mode==="parallel"} on:click={() => scrForm.mode="parallel"}>
                            ⇶ Parallel <span class="mode-hint">all at once</span>
                        </button>
                    </div>
                </div>

                <div class="steps-section">
                    <div class="steps-header">
                        <span class="section-label">Steps ({scrForm.steps.length})</span>
                        <button class="add-step-btn" on:click={addStep}><PlusCircled size={13} /> Add Step</button>
                    </div>
                    {#if scrForm.steps.length === 0}
                        <div class="no-steps-hint">No steps yet — click "Add Step" to build your automation</div>
                    {/if}
                    {#each scrForm.steps as step, i}
                        <div class="step-editor">
                            <div class="step-controls">
                                <button class="step-move" on:click={() => moveStep(i,-1)} disabled={i===0}>▲</button>
                                <span class="step-num">{i+1}</span>
                                <button class="step-move" on:click={() => moveStep(i,1)} disabled={i===scrForm.steps.length-1}>▼</button>
                            </div>
                            <div class="step-body">
                                <div class="step-type-row">
                                    <select bind:value={step.step_type} class="step-type-select">
                                        <option value="command_ref">⚡ Command Reference</option>
                                        <option value="delay">⏱ Delay / Pause</option>
                                        <option value="custom">&gt;_ Custom Shell</option>
                                        <option value="spotify">🎵 Spotify</option>
                                    </select>
                                    <input class="step-label-input" bind:value={step.label} placeholder="Label (optional)" />
                                </div>
                                {#if step.step_type === "command_ref"}
                                    <div class="step-fields">
                                        <div class="field-mini">
                                            <label>Pack</label>
                                            <select bind:value={step.pack}>
                                                {#each packs as p}<option value={p.pack_name}>{p.pack_name}</option>{/each}
                                                {#if packs.length===0}<option value="">— no packs —</option>{/if}
                                            </select>
                                        </div>
                                        <div class="field-mini">
                                            <label>Command</label>
                                            <select bind:value={step.command_id}>
                                                {#each packCmds(step.pack) as cmd}<option value={cmd.id}>{cmd.id}</option>{/each}
                                                {#if packCmds(step.pack).length===0}<option value="">— select pack first —</option>{/if}
                                            </select>
                                        </div>
                                    </div>
                                {:else if step.step_type === "delay"}
                                    <div class="step-fields">
                                        <div class="field-mini">
                                            <label>Duration (ms)</label>
                                            <input type="number" bind:value={step.delay_ms} min="0" step="500" placeholder="2000" />
                                        </div>
                                    </div>
                                {:else if step.step_type === "custom"}
                                    <div class="step-fields">
                                        <div class="field-mini" style="flex:0.5">
                                            <label>Command</label>
                                            <input bind:value={step.cli_cmd} placeholder="powershell" />
                                        </div>
                                        <div class="field-mini">
                                            <label>Arguments <span class="hint">(comma-sep)</span></label>
                                            <input bind:value={step.cli_args} placeholder="-NoProfile, -Command, echo hi" />
                                        </div>
                                    </div>
                                {:else if step.step_type === "spotify"}
                                    <div class="step-fields">
                                        <div class="field-mini" style="flex:0.5">
                                            <label>Action</label>
                                            <select bind:value={step.spotify_action}>
                                                <option value="play_track">▶ Play Track</option>
                                                <option value="pause">⏸ Pause</option>
                                                <option value="next">⏭ Next</option>
                                            </select>
                                        </div>
                                        {#if step.spotify_action === "play_track"}
                                            <div class="field-mini">
                                                <label>Track ID</label>
                                                <input bind:value={step.spotify_track_id} placeholder="4uLU6hMCjMI75M1A2tKUQC" />
                                            </div>
                                        {/if}
                                    </div>
                                {/if}
                            </div>
                            <button class="step-delete" on:click={() => removeStep(i)} title="Remove step"><Cross2 size={12} /></button>
                        </div>
                    {/each}
                </div>

                <details class="voice-section">
                    <summary>Voice triggers <span class="hint">(optional)</span></summary>
                    <div class="voice-fields">
                        <div class="field-row">
                            <div class="field">
                                <label>Phrases RU</label>
                                <textarea bind:value={scrForm.phrases_ru} rows="2" placeholder="режим работы, запусти утро"></textarea>
                            </div>
                            <div class="field">
                                <label>Phrases EN</label>
                                <textarea bind:value={scrForm.phrases_en} rows="2" placeholder="work mode"></textarea>
                            </div>
                        </div>
                        <div class="field">
                            <label>Regex <span class="hint">(optional)</span></label>
                            <input bind:value={scrForm.patterns} placeholder="режим|routine" />
                        </div>
                        <div class="field">
                            <label>Response Sound <span class="hint">(optional — overrides voice pack)</span></label>
                            <select bind:value={scrForm.response_sound} class="select-input">
                                <option value="">— random from voice pack —</option>
                                {#each availableSoundFiles as f}
                                    <option value={f}>{f.split("/").pop()}</option>
                                {/each}
                            </select>
                        </div>
                    </div>
                </details>

                {#if scrForm.formError}<div class="form-error">{scrForm.formError}</div>{/if}
            </div>
            <div class="modal-footer">
                <Button color="gray" radius="md" size="sm" uppercase on:click={closeScrModal}>Cancel</Button>
                <Button color="lime" radius="md" size="sm" uppercase ripple disabled={scrSaving} on:click={saveScript}>
                    {scrSaving ? "Saving…" : (scrModalMode === "edit" ? "Save Changes" : "Create Script")}
                </Button>
            </div>
        </div>
    </div>
{/if}

<style lang="scss">
// ── Tab bar ───────────────────────────────────────────────────────────────────

.tab-bar {
    display: flex;
    gap: 0;
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 10px;
    padding: 4px;
    margin-bottom: 1.1rem;
    width: fit-content;
}
.tab {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 7px;
    color: rgba(255,255,255,0.4);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 6px 18px;
    text-transform: uppercase;
    transition: all 0.15s;
    &:hover { color: rgba(255,255,255,0.7); }
    &.active {
        background: rgba(255,255,255,0.07);
        border-color: rgba(255,255,255,0.12);
        color: #fff;
    }
}
.tab-count {
    background: rgba(255,255,255,0.12);
    border-radius: 10px;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 6px;
}
.tab.active .tab-count { background: rgba(138,200,50,0.25); color: #8AC832; }

// ── Page header ───────────────────────────────────────────────────────────────

.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
}
.page-sub {
    margin: 0;
    font-size: 0.75rem;
    color: rgba(255,255,255,0.35);
}
.header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
}
.icon-btn {
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px;
    color: rgba(255,255,255,0.5);
    cursor: pointer;
    padding: 5px 7px;
    display: flex;
    align-items: center;
    transition: all 0.15s;
    &:hover { color: #fff; background: rgba(255,255,255,0.1); }
}

// ── Toasts ────────────────────────────────────────────────────────────────────

.toast {
    padding: 0.6rem 1rem;
    border-radius: 8px;
    font-size: 0.8rem;
    margin-bottom: 0.5rem;
    &.success { background: rgba(138,200,50,0.15); border: 1px solid rgba(138,200,50,0.4); color: #8AC832; }
    &.error   { background: rgba(239,68,68,0.12); border: 1px solid rgba(239,68,68,0.3); color: #f87171; }
}
.empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: rgba(255,255,255,0.3);
    font-size: 0.85rem;
    p { margin: 0 0 0.5rem; }
}
.empty-hint {
    color: rgba(255,255,255,0.2) !important;
    font-size: 0.75rem !important;
    font-style: italic;
    max-width: 340px;
    margin: 0 auto 1rem !important;
}

// ── Commands: pack section ────────────────────────────────────────────────────

.pack-section {
    margin-bottom: 1.25rem;
    background: rgba(20,30,35,0.6);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 10px;
    overflow: hidden;
}
.pack-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.55rem 0.9rem;
    background: rgba(255,255,255,0.04);
    border-bottom: 1px solid rgba(255,255,255,0.06);
}
.pack-header-right { display: flex; align-items: center; gap: 0.4rem; }
.pack-name {
    font-size: 0.78rem; font-weight: 600;
    color: rgba(82,254,254,0.8);
    letter-spacing: 0.05em; text-transform: uppercase;
}
.icon-btn-sm {
    background: none; border: 1px solid transparent;
    border-radius: 5px; color: rgba(255,255,255,0.3);
    cursor: pointer; padding: 3px 6px;
    display: flex; align-items: center; transition: all 0.15s;
    &:hover { color: #8AC832; border-color: rgba(138,200,50,0.3); }
}
.delete-pack-btn {
    display: flex; align-items: center; gap: 0.35rem;
    background: none; border: 1px solid transparent;
    border-radius: 5px; color: rgba(255,255,255,0.25);
    cursor: pointer; font-size: 0.7rem; padding: 3px 8px; transition: all 0.15s;
    &:hover { color: #f87171; border-color: rgba(248,113,113,0.3); }
    &.confirm { color: #f87171; border-color: rgba(248,113,113,0.5); background: rgba(239,68,68,0.1); }
}

// ── Commands: table ───────────────────────────────────────────────────────────

.cmd-table {
    width: 100%; border-collapse: collapse; font-size: 0.78rem;
    th {
        text-align: left; padding: 0.45rem 0.9rem;
        color: rgba(255,255,255,0.3); font-weight: 500; font-size: 0.7rem;
        text-transform: uppercase; letter-spacing: 0.05em;
        border-bottom: 1px solid rgba(255,255,255,0.05);
    }
    td {
        padding: 0.55rem 0.9rem; color: rgba(255,255,255,0.8);
        border-bottom: 1px solid rgba(255,255,255,0.04); vertical-align: middle;
    }
    tr:last-child td { border-bottom: none; }
    tr:hover td { background: rgba(255,255,255,0.02); }
}
.th-actions { width: 64px; }
.cell-id { font-family: monospace; color: #fff !important; font-weight: 600; min-width: 120px; }
.cell-phrases {
    color: rgba(255,255,255,0.5) !important;
    max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.cell-path {
    color: rgba(255,255,255,0.35) !important;
    max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-family: monospace; font-size: 0.7rem;
}
.cell-actions {
    display: flex; align-items: center; gap: 0.25rem; padding-right: 0.5rem !important;
}
.badge {
    display: inline-block; padding: 2px 7px;
    border-radius: 4px; border: 1px solid;
    font-size: 0.65rem; font-weight: 700;
    letter-spacing: 0.06em; text-transform: uppercase;
}
.row-btn {
    background: none; border: 1px solid transparent;
    border-radius: 5px; cursor: pointer;
    padding: 3px 5px; display: flex; align-items: center;
    transition: all 0.15s; opacity: 0;
    tr:hover & { opacity: 1; }
}
.edit-btn {
    color: rgba(255,255,255,0.3);
    &:hover { color: #52fefe; border-color: rgba(82,254,254,0.3); }
}
.trash-btn {
    color: rgba(255,255,255,0.3);
    &:hover { color: #f87171; border-color: rgba(248,113,113,0.3); }
    &.confirm { color: #f87171; border-color: rgba(248,113,113,0.5); background: rgba(239,68,68,0.1); opacity: 1; }
}

// ── Scripts: grid ─────────────────────────────────────────────────────────────

.script-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 0.9rem;
}
.script-card {
    background: rgba(20,30,35,0.7);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 12px; overflow: hidden;
    display: flex; flex-direction: column;
    transition: border-color 0.15s;
    &:hover { border-color: rgba(255,255,255,0.14); }
}
.card-header { padding: 0.85rem 1rem 0.6rem; }
.card-title-row {
    display: flex; align-items: center; gap: 0.5rem; min-width: 0;
}
.card-name {
    font-size: 0.88rem; font-weight: 700; color: #fff;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    flex: 1; min-width: 0;
}
.mode-badge {
    flex-shrink: 0; font-size: 0.6rem; font-weight: 700;
    letter-spacing: 0.06em; text-transform: uppercase;
    border: 1px solid; border-radius: 4px; padding: 2px 6px;
}
.card-desc {
    margin: 0.3rem 0 0; font-size: 0.72rem; color: rgba(255,255,255,0.4);
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.steps-preview {
    padding: 0 1rem 0.6rem; display: flex; flex-direction: column; gap: 0.2rem; flex: 1;
}
.step-chip {
    display: flex; align-items: center; gap: 0.4rem;
    font-size: 0.72rem; color: rgba(255,255,255,0.5);
    background: rgba(255,255,255,0.04);
    border-radius: 5px; padding: 3px 8px; min-width: 0;
}
.step-icon { flex-shrink: 0; }
.step-text { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
.step-chip.more { color: rgba(255,255,255,0.25); font-style: italic; }
.no-steps { font-size: 0.72rem; color: rgba(255,255,255,0.2); font-style: italic; padding: 0 0 0.4rem; }
.card-footer {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.5rem 1rem;
    border-top: 1px solid rgba(255,255,255,0.05);
    background: rgba(255,255,255,0.02); gap: 0.5rem;
}
.step-count { font-size: 0.68rem; color: rgba(255,255,255,0.25); flex-shrink: 0; }
.card-actions { display: flex; align-items: center; gap: 0.3rem; flex-shrink: 0; }
.action-btn {
    display: flex; align-items: center; gap: 0.25rem;
    background: none; border: 1px solid transparent;
    border-radius: 5px; cursor: pointer;
    font-size: 0.7rem; padding: 3px 8px;
    transition: all 0.15s; white-space: nowrap;
    &:disabled { opacity: 0.4; cursor: default; }
}
.run-btn {
    color: #8AC832; border-color: rgba(138,200,50,0.3); background: rgba(138,200,50,0.08);
    &:hover:not(:disabled) { background: rgba(138,200,50,0.18); }
}
.edit-btn-card {
    color: rgba(255,255,255,0.35);
    &:hover { color: #52fefe; border-color: rgba(82,254,254,0.3); }
}
.trash-btn-card {
    color: rgba(255,255,255,0.25);
    &:hover { color: #f87171; border-color: rgba(248,113,113,0.3); }
    &.confirm { color: #f87171; border-color: rgba(248,113,113,0.5); background: rgba(239,68,68,0.1); }
}

// ── Modal (shared) ────────────────────────────────────────────────────────────

.modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.75); z-index: 100;
    display: flex; align-items: center; justify-content: center; backdrop-filter: blur(4px);
}
.modal {
    background: #0f1a1e; border: 1px solid rgba(255,255,255,0.12);
    border-radius: 14px; width: min(600px, 95vw); max-height: 90vh;
    overflow-y: auto; display: flex; flex-direction: column;
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
    &::-webkit-scrollbar { width: 5px; }
    &::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); border-radius: 3px; }
}
.modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 1rem 1.25rem 0.75rem; border-bottom: 1px solid rgba(255,255,255,0.08);
    h3 { margin: 0; font-size: 1rem; font-weight: 700; color: #fff;
         text-transform: uppercase; letter-spacing: 0.06em;
         display: flex; align-items: center; }
}
.close-btn {
    background: none; border: none; color: rgba(255,255,255,0.4);
    cursor: pointer; padding: 4px; display: flex; align-items: center;
    border-radius: 4px; transition: color 0.15s;
    &:hover { color: #fff; }
}
.modal-body {
    padding: 1rem 1.25rem; display: flex; flex-direction: column; gap: 0.75rem;
}
.modal-footer {
    display: flex; justify-content: flex-end; gap: 0.5rem;
    padding: 0.75rem 1.25rem; border-top: 1px solid rgba(255,255,255,0.08);
}

// ── Form fields ───────────────────────────────────────────────────────────────

.field {
    display: flex; flex-direction: column; gap: 0.3rem;
    label {
        font-size: 0.72rem; font-weight: 600; color: rgba(255,255,255,0.65);
        text-transform: uppercase; letter-spacing: 0.04em;
    }
    .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.3); letter-spacing: 0; }
    input, textarea, select {
        background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1);
        border-radius: 6px; color: #fff; font-size: 0.82rem; padding: 0.5rem 0.7rem;
        outline: none; transition: border-color 0.15s; resize: vertical; font-family: inherit;
        &::placeholder { color: rgba(255,255,255,0.2); }
        &:focus { border-color: rgba(82,254,254,0.4); }
    }
    select option { background: #0f1a1e; color: #fff; }
}
.select-input { cursor: pointer; appearance: auto; }
.readonly-pack {
    background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.07);
    border-radius: 6px; color: rgba(82,254,254,0.7);
    font-size: 0.82rem; font-family: monospace; padding: 0.5rem 0.7rem;
}
.field-row { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
.type-tabs { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.type-tab {
    background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1);
    border-radius: 6px; color: rgba(255,255,255,0.5); cursor: pointer;
    font-size: 0.75rem; font-weight: 600; padding: 5px 12px;
    text-transform: uppercase; letter-spacing: 0.04em; transition: all 0.15s;
    &:hover { color: #fff; border-color: rgba(255,255,255,0.25); }
    &.active { background: rgba(138,200,50,0.15); border-color: rgba(138,200,50,0.5); color: #8AC832; }
}
.chrome-hint {
    font-size: 0.7rem; color: rgba(255,255,255,0.3); padding: 0.4rem 0.6rem;
    background: rgba(255,255,255,0.03); border-radius: 5px; border: 1px solid rgba(255,255,255,0.06);
    overflow-x: auto; code { font-family: monospace; color: rgba(82,254,254,0.6); }
}
.form-error {
    background: rgba(239,68,68,0.1); border: 1px solid rgba(239,68,68,0.3);
    border-radius: 6px; color: #f87171; font-size: 0.78rem; padding: 0.5rem 0.75rem;
}

// ── Scripts modal: mode tabs ──────────────────────────────────────────────────

.mode-tabs { display: flex; gap: 0.5rem; }
.mode-tab {
    flex: 1; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px; color: rgba(255,255,255,0.4); cursor: pointer;
    font-size: 0.8rem; font-weight: 600; padding: 0.6rem 0.8rem;
    text-align: left; transition: all 0.15s;
    display: flex; flex-direction: column; gap: 0.15rem;
    &:hover { color: #fff; border-color: rgba(255,255,255,0.2); }
    &.active { background: rgba(82,254,254,0.08); border-color: rgba(82,254,254,0.35); color: #52fefe;
               .mode-hint { color: rgba(82,254,254,0.5); } }
}
.mode-hint { font-size: 0.65rem; font-weight: 400; letter-spacing: 0; color: rgba(255,255,255,0.25); }

// ── Scripts modal: step builder ───────────────────────────────────────────────

.steps-section {
    background: rgba(255,255,255,0.02); border: 1px solid rgba(255,255,255,0.07);
    border-radius: 10px; overflow: hidden;
}
.steps-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.6rem 0.9rem; border-bottom: 1px solid rgba(255,255,255,0.06);
    background: rgba(255,255,255,0.03);
}
.section-label {
    font-size: 0.72rem; font-weight: 600; color: rgba(255,255,255,0.5);
    text-transform: uppercase; letter-spacing: 0.05em;
}
.add-step-btn {
    display: flex; align-items: center; gap: 0.3rem;
    background: none; border: 1px solid rgba(138,200,50,0.3);
    border-radius: 5px; color: #8AC832; cursor: pointer;
    font-size: 0.7rem; font-weight: 600; padding: 3px 10px; transition: all 0.15s;
    &:hover { background: rgba(138,200,50,0.1); }
}
.no-steps-hint {
    padding: 1.2rem; text-align: center; font-size: 0.75rem;
    color: rgba(255,255,255,0.2); font-style: italic;
}
.step-editor {
    display: flex; align-items: flex-start; gap: 0.6rem;
    padding: 0.7rem 0.9rem; border-bottom: 1px solid rgba(255,255,255,0.04);
    &:last-child { border-bottom: none; }
}
.step-controls {
    display: flex; flex-direction: column; align-items: center;
    gap: 0; flex-shrink: 0; padding-top: 2px;
}
.step-num {
    font-size: 0.65rem; color: rgba(255,255,255,0.25); width: 18px;
    text-align: center; line-height: 1.6;
}
.step-move {
    background: none; border: none; color: rgba(255,255,255,0.2);
    cursor: pointer; font-size: 0.6rem; line-height: 1; padding: 1px 4px; transition: color 0.1s;
    &:hover:not(:disabled) { color: #fff; }
    &:disabled { opacity: 0.2; cursor: default; }
}
.step-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.step-type-row { display: flex; gap: 0.4rem; align-items: center; }
.step-type-select {
    background: rgba(255,255,255,0.06); border: 1px solid rgba(255,255,255,0.12);
    border-radius: 5px; color: #fff; font-size: 0.75rem; padding: 4px 8px;
    outline: none; cursor: pointer; flex-shrink: 0;
    option { background: #0f1a1e; }
    &:focus { border-color: rgba(82,254,254,0.3); }
}
.step-label-input {
    flex: 1; min-width: 0;
    background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.07);
    border-radius: 5px; color: rgba(255,255,255,0.6);
    font-size: 0.75rem; padding: 4px 8px; outline: none; font-family: inherit;
    &::placeholder { color: rgba(255,255,255,0.18); }
    &:focus { border-color: rgba(82,254,254,0.3); }
}
.step-fields { display: flex; gap: 0.4rem; flex-wrap: wrap; }
.field-mini {
    display: flex; flex-direction: column; gap: 0.2rem; flex: 1; min-width: 120px;
    label { font-size: 0.65rem; font-weight: 600; color: rgba(255,255,255,0.4);
            text-transform: uppercase; letter-spacing: 0.04em; }
    .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.25); letter-spacing: 0; }
    input, select {
        background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.09);
        border-radius: 5px; color: #fff; font-size: 0.78rem; padding: 4px 8px;
        outline: none; font-family: inherit;
        &::placeholder { color: rgba(255,255,255,0.2); }
        &:focus { border-color: rgba(82,254,254,0.35); }
    }
    select option { background: #0f1a1e; }
}
.step-delete {
    flex-shrink: 0; background: none; border: none; color: rgba(255,255,255,0.2);
    cursor: pointer; padding: 4px; display: flex; align-items: center;
    border-radius: 4px; transition: color 0.1s; margin-top: 2px;
    &:hover { color: #f87171; }
}
.voice-section {
    border: 1px solid rgba(255,255,255,0.06); border-radius: 8px; overflow: hidden;
    summary {
        padding: 0.6rem 0.9rem; cursor: pointer;
        font-size: 0.72rem; font-weight: 600; color: rgba(255,255,255,0.5);
        text-transform: uppercase; letter-spacing: 0.04em;
        background: rgba(255,255,255,0.03); user-select: none;
        .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.25); letter-spacing: 0; }
        &:hover { color: rgba(255,255,255,0.7); }
    }
    &[open] summary { border-bottom: 1px solid rgba(255,255,255,0.06); }
}
.voice-fields { padding: 0.75rem 0.9rem; display: flex; flex-direction: column; gap: 0.6rem; }
</style>
