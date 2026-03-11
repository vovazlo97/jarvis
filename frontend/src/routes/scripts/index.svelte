<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    import { Button, Space } from "@svelteuidev/core"
    import { PlusCircled, Trash, Reload, Cross2, Play, Pencil1,
             LightningBolt } from "radix-icons-svelte"

    import HDivider from "@/components/elements/HDivider.svelte"
    import Footer from "@/components/Footer.svelte"

    // ── Types ────────────────────────────────────────────────────────────────────

    interface ScriptStep {
        step_type: "command_ref" | "delay" | "custom" | "spotify"
        label: string
        pack: string
        command_id: string
        delay_ms: number
        cli_cmd: string
        cli_args: string[]
        spotify_action: string
        spotify_track_id: string
    }

    interface Script {
        id: string
        name: string
        description: string
        mode: "sequential" | "parallel"
        steps: ScriptStep[]
        phrases_ru: string[]
        phrases_en: string[]
        patterns: string[]
        sounds_ru: string[]
        response_sound: string
    }

    interface CommandPack {
        pack_name: string
        commands: Array<{ id: string; type: string }>
    }

    // ── State ────────────────────────────────────────────────────────────────────

    let scripts: Script[] = []
    let packs: CommandPack[] = []
    let availableSoundFiles: string[] = []
    let loading = true
    let globalError = ""
    let successMsg = ""
    let runningId = ""
    let showModal = false
    let saving = false
    let deleteTarget = ""

    // ── Form ─────────────────────────────────────────────────────────────────────

    type ModalMode = "add" | "edit"
    let modalMode: ModalMode = "add"

    interface Form {
        id: string
        name: string
        description: string
        mode: "sequential" | "parallel"
        steps: FormStep[]
        phrases_ru: string
        phrases_en: string
        patterns: string
        response_sound: string
        formError: string
    }

    interface FormStep {
        step_type: "command_ref" | "delay" | "custom" | "spotify"
        label: string
        pack: string
        command_id: string
        delay_ms: string   // string for input binding
        cli_cmd: string
        cli_args: string   // comma-separated for input binding
        spotify_action: string
        spotify_track_id: string
    }

    let idTouched = false
    let originalId: string | undefined = undefined
    let form: Form = emptyForm()

    function emptyForm(): Form {
        return {
            id: "", name: "", description: "", mode: "sequential",
            steps: [], phrases_ru: "", phrases_en: "", patterns: "",
            response_sound: "", formError: "",
        }
    }

    function emptyStep(): FormStep {
        return {
            step_type: "command_ref", label: "",
            pack: packs[0]?.pack_name ?? "",
            command_id: "", delay_ms: "500", cli_cmd: "", cli_args: "",
            spotify_action: "play_track", spotify_track_id: "",
        }
    }

    $: if (form.name && !idTouched) {
        form.id = form.name.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "")
    }

    // ── API ──────────────────────────────────────────────────────────────────────

    async function loadAll() {
        loading = true
        globalError = ""
        try {
            [scripts, packs, availableSoundFiles] = await Promise.all([
                invoke<Script[]>("list_scripts"),
                invoke<CommandPack[]>("list_command_packs"),
                invoke<string[]>("list_sound_files"),
            ])
        } catch (e) {
            globalError = String(e)
        }
        loading = false
    }

    async function runScript(id: string) {
        runningId = id
        try {
            await invoke("run_script", { scriptId: id })
            flash(`Script "${id}" launched`)
        } catch (e) {
            globalError = String(e)
        }
        runningId = ""
    }

    async function deleteScript(id: string) {
        if (deleteTarget !== id) {
            deleteTarget = id
            setTimeout(() => { deleteTarget = "" }, 3000)
            return
        }
        deleteTarget = ""
        try {
            await invoke("delete_script", { scriptId: id })
            await loadAll()
            invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
        } catch (e) {
            globalError = String(e)
        }
    }

    async function save() {
        form.formError = ""
        if (!form.id.trim())   { form.formError = "Script ID is required"; return }
        if (!form.name.trim()) { form.formError = "Script name is required"; return }
        if (form.steps.length === 0) { form.formError = "Add at least one step"; return }

        const steps: ScriptStep[] = form.steps.map(s => ({
            step_type: s.step_type,
            label: s.label.trim(),
            pack: s.pack,
            command_id: s.command_id,
            delay_ms: parseInt(s.delay_ms) || 500,
            cli_cmd: s.cli_cmd.trim(),
            cli_args: s.cli_args.split(",").map(x => x.trim()).filter(Boolean),
            spotify_action: s.spotify_action,
            spotify_track_id: s.spotify_track_id.trim(),
        }))

        const payload: Script = {
            id: form.id.trim(),
            name: form.name.trim(),
            description: form.description.trim(),
            mode: form.mode,
            steps,
            phrases_ru: form.phrases_ru.split(",").map(x => x.trim()).filter(Boolean),
            phrases_en: form.phrases_en.split(",").map(x => x.trim()).filter(Boolean),
            patterns: form.patterns.split(",").map(x => x.trim()).filter(Boolean),
            sounds_ru: ["ok1", "ok2", "ok3"],
            response_sound: form.response_sound,
        }

        saving = true
        try {
            await invoke("save_script", {
                script: payload,
                oldId: modalMode === "edit" ? originalId : undefined,
            })
            flash(modalMode === "edit"
                ? `Script "${payload.name}" updated`
                : `Script "${payload.name}" created`)
            closeModal()
            await loadAll()
            invoke("reload_jarvis_commands").catch(() => {/* jarvis-app may not be running */})
        } catch (e) {
            form.formError = String(e)
        }
        saving = false
    }

    // ── Modal helpers ─────────────────────────────────────────────────────────────

    function openAddModal() {
        modalMode = "add"
        form = emptyForm()
        idTouched = false
        showModal = true
    }

    function openEditModal(s: Script) {
        modalMode = "edit"
        originalId = s.id
        idTouched = true
        form = {
            id: s.id,
            name: s.name,
            description: s.description,
            mode: s.mode,
            steps: s.steps.map(st => ({
                step_type: st.step_type,
                label: st.label,
                pack: st.pack,
                command_id: st.command_id,
                delay_ms: String(st.delay_ms),
                cli_cmd: st.cli_cmd,
                cli_args: st.cli_args.join(", "),
                spotify_action: st.spotify_action || "play_track",
                spotify_track_id: st.spotify_track_id || "",
            })),
            phrases_ru: s.phrases_ru.join(", "),
            phrases_en: s.phrases_en.join(", "),
            patterns: s.patterns.join(", "),
            response_sound: s.response_sound || "",
            formError: "",
        }
        showModal = true
    }

    function closeModal() {
        showModal = false
        form = emptyForm()
        idTouched = false
        originalId = undefined
    }

    function addStep() {
        form.steps = [...form.steps, emptyStep()]
    }

    function removeStep(i: number) {
        form.steps = form.steps.filter((_, idx) => idx !== i)
    }

    function moveStep(i: number, dir: -1 | 1) {
        const j = i + dir
        if (j < 0 || j >= form.steps.length) return
        const arr = [...form.steps]
        ;[arr[i], arr[j]] = [arr[j], arr[i]]
        form.steps = arr
    }

    // ── Helpers ───────────────────────────────────────────────────────────────────

    function flash(msg: string) {
        successMsg = msg
        setTimeout(() => { successMsg = "" }, 4000)
    }

    function modeColor(mode: string) {
        return mode === "parallel" ? "#a78bfa" : "#52fefe"
    }

    function modeLabel(mode: string) {
        return mode === "parallel" ? "Parallel" : "Sequential"
    }

    function stepSummary(s: ScriptStep): string {
        switch (s.step_type) {
            case "command_ref": return `${s.pack} › ${s.command_id}`
            case "delay":       return `Wait ${s.delay_ms} ms`
            case "custom":      return s.cli_cmd || "Shell command"
            case "spotify": {
                if (s.spotify_action === "pause") return "Spotify: Pause"
                if (s.spotify_action === "next")  return "Spotify: Next"
                return `Spotify: ${s.spotify_track_id || "play track"}`
            }
            default:            return "Unknown"
        }
    }

    function stepIcon(type: string): string {
        if (type === "command_ref") return "⚡"
        if (type === "delay")       return "⏱"
        if (type === "custom")      return ">"
        if (type === "spotify")     return "🎵"
        return "?"
    }

    // Pack commands for a given pack name
    function packCommands(packName: string): Array<{id: string}> {
        return packs.find(p => p.pack_name === packName)?.commands ?? []
    }

    onMount(loadAll)
</script>

<!-- ── Toast ─────────────────────────────────────────────────────────────────── -->
{#if successMsg}
    <div class="toast success">{successMsg}</div>
{/if}

<Space h="xl" />

<!-- ── Header ────────────────────────────────────────────────────────────────── -->
<div class="page-header">
    <div>
        <h2 class="page-title">Scripts</h2>
        <p class="page-sub">
            {#if loading}Loading…{:else}{scripts.length} automation scripts{/if}
        </p>
    </div>
    <div class="header-actions">
        <button class="icon-btn" title="Refresh" on:click={loadAll}>
            <Reload size={16} />
        </button>
        <Button color="lime" radius="md" size="sm" uppercase ripple on:click={openAddModal}>
            <PlusCircled size={14} />
            &nbsp;New Script
        </Button>
    </div>
</div>

{#if globalError}
    <div class="toast error">{globalError}</div>
{/if}

<Space h="md" />

<!-- ── Script cards ───────────────────────────────────────────────────────────── -->
{#if loading}
    <div class="empty-state">Loading scripts…</div>
{:else if scripts.length === 0}
    <div class="empty-state">
        <p>No scripts yet.</p>
        <Button color="lime" radius="md" size="sm" uppercase on:click={openAddModal}>
            Create your first script
        </Button>
    </div>
{:else}
    <div class="script-grid">
        {#each scripts as s}
            <div class="script-card">
                <!-- Card header -->
                <div class="card-header">
                    <div class="card-title-row">
                        <span class="card-name" title={s.name}>{s.name}</span>
                        <span class="mode-badge" style="color:{modeColor(s.mode)}; border-color:{modeColor(s.mode)}33">
                            {modeLabel(s.mode)}
                        </span>
                    </div>
                    {#if s.description}
                        <p class="card-desc" title={s.description}>{s.description}</p>
                    {/if}
                </div>

                <!-- Steps preview -->
                <div class="steps-preview">
                    {#each s.steps.slice(0, 4) as step}
                        <div class="step-chip">
                            <span class="step-icon">{stepIcon(step.step_type)}</span>
                            <span class="step-text" title={stepSummary(step)}>{stepSummary(step)}</span>
                        </div>
                    {/each}
                    {#if s.steps.length > 4}
                        <div class="step-chip more">+{s.steps.length - 4} more</div>
                    {/if}
                    {#if s.steps.length === 0}
                        <span class="no-steps">No steps</span>
                    {/if}
                </div>

                <!-- Card footer: action buttons -->
                <div class="card-footer">
                    <span class="step-count">{s.steps.length} step{s.steps.length !== 1 ? "s" : ""}</span>
                    <div class="card-actions">
                        <button
                            class="action-btn run-btn"
                            disabled={runningId === s.id}
                            title="Run script"
                            on:click={() => runScript(s.id)}
                        >
                            <Play size={13} />
                            {runningId === s.id ? "Running…" : "Run"}
                        </button>
                        <button
                            class="action-btn edit-btn"
                            title="Edit script"
                            on:click={() => openEditModal(s)}
                        >
                            <Pencil1 size={13} />
                        </button>
                        <button
                            class="action-btn trash-btn"
                            class:confirm={deleteTarget === s.id}
                            title={deleteTarget === s.id ? "Confirm delete" : "Delete script"}
                            on:click={() => deleteScript(s.id)}
                        >
                            <Trash size={13} />
                            {deleteTarget === s.id ? "Sure?" : ""}
                        </button>
                    </div>
                </div>
            </div>
        {/each}
    </div>
{/if}

<HDivider />
<Footer />

<!-- ── Modal ──────────────────────────────────────────────────────────────────── -->
{#if showModal}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click|self={closeModal}>
        <div class="modal">

            <!-- Header -->
            <div class="modal-header">
                <h3>
                    <LightningBolt size={16} style="margin-right:6px;opacity:.7" />
                    {modalMode === "edit" ? "Edit Script" : "New Script"}
                </h3>
                <button class="close-btn" on:click={closeModal}><Cross2 size={16} /></button>
            </div>

            <div class="modal-body">

                <!-- Identity -->
                <div class="field-row">
                    <div class="field">
                        <label>Script Name</label>
                        <input bind:value={form.name} placeholder="Morning Routine" />
                    </div>
                    <div class="field">
                        <label>ID <span class="hint">(auto)</span></label>
                        <input bind:value={form.id}
                               on:input={() => idTouched = true}
                               placeholder="morning_routine" />
                    </div>
                </div>

                <div class="field">
                    <label>Description <span class="hint">(optional)</span></label>
                    <input bind:value={form.description} placeholder="What this script does…" />
                </div>

                <!-- Execution mode -->
                <div class="field">
                    <label>Execution Mode</label>
                    <div class="mode-tabs">
                        <button
                            class="mode-tab"
                            class:active={form.mode === "sequential"}
                            on:click={() => form.mode = "sequential"}
                        >
                            ▶▶ Sequential
                            <span class="mode-hint">steps run one after another</span>
                        </button>
                        <button
                            class="mode-tab"
                            class:active={form.mode === "parallel"}
                            on:click={() => form.mode = "parallel"}
                        >
                            ⇶ Parallel
                            <span class="mode-hint">all steps start at once</span>
                        </button>
                    </div>
                </div>

                <!-- Steps builder -->
                <div class="steps-section">
                    <div class="steps-header">
                        <span class="section-label">Steps ({form.steps.length})</span>
                        <button class="add-step-btn" on:click={addStep}>
                            <PlusCircled size={13} /> Add Step
                        </button>
                    </div>

                    {#if form.steps.length === 0}
                        <div class="no-steps-hint">
                            No steps yet — click "Add Step" to build your automation
                        </div>
                    {/if}

                    {#each form.steps as step, i}
                        <div class="step-editor">
                            <div class="step-controls">
                                <button class="step-move" on:click={() => moveStep(i, -1)} disabled={i === 0}>▲</button>
                                <span class="step-num">{i + 1}</span>
                                <button class="step-move" on:click={() => moveStep(i, 1)} disabled={i === form.steps.length - 1}>▼</button>
                            </div>

                            <div class="step-body">
                                <!-- Type selector -->
                                <div class="step-type-row">
                                    <select bind:value={step.step_type} class="step-type-select">
                                        <option value="command_ref">⚡ Command Reference</option>
                                        <option value="delay">⏱ Delay / Pause</option>
                                        <option value="custom">&gt;_ Custom Shell</option>
                                        <option value="spotify">🎵 Spotify</option>
                                    </select>
                                    <input class="step-label-input"
                                           bind:value={step.label}
                                           placeholder="Label (optional)" />
                                </div>

                                <!-- Type-specific fields -->
                                {#if step.step_type === "command_ref"}
                                    <div class="step-fields">
                                        <div class="field-mini">
                                            <label>Pack</label>
                                            <select bind:value={step.pack}>
                                                {#each packs as p}
                                                    <option value={p.pack_name}>{p.pack_name}</option>
                                                {/each}
                                                {#if packs.length === 0}
                                                    <option value="">— no packs loaded —</option>
                                                {/if}
                                            </select>
                                        </div>
                                        <div class="field-mini">
                                            <label>Command</label>
                                            <select bind:value={step.command_id}>
                                                {#each packCommands(step.pack) as cmd}
                                                    <option value={cmd.id}>{cmd.id}</option>
                                                {/each}
                                                {#if packCommands(step.pack).length === 0}
                                                    <option value="">— no commands —</option>
                                                {/if}
                                            </select>
                                        </div>
                                    </div>

                                {:else if step.step_type === "delay"}
                                    <div class="step-fields">
                                        <div class="field-mini">
                                            <label>Duration (ms)</label>
                                            <input type="number" bind:value={step.delay_ms}
                                                   min="0" step="500" placeholder="2000" />
                                        </div>
                                    </div>

                                {:else if step.step_type === "custom"}
                                    <div class="step-fields">
                                        <div class="field-mini" style="flex:0.5">
                                            <label>Command</label>
                                            <input bind:value={step.cli_cmd} placeholder="powershell" />
                                        </div>
                                        <div class="field-mini" style="flex:1">
                                            <label>Arguments <span class="hint">(comma-separated)</span></label>
                                            <input bind:value={step.cli_args}
                                                   placeholder="-NoProfile, -Command, echo Hello" />
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
                                            <div class="field-mini" style="flex:1">
                                                <label>Track ID</label>
                                                <input bind:value={step.spotify_track_id}
                                                       placeholder="4uLU6hMCjMI75M1A2tKUQC" />
                                            </div>
                                        {/if}
                                    </div>
                                {/if}
                            </div>

                            <button class="step-delete" on:click={() => removeStep(i)} title="Remove step">
                                <Cross2 size={12} />
                            </button>
                        </div>
                    {/each}
                </div>

                <!-- Voice triggers (collapsible section) -->
                <details class="voice-section">
                    <summary>Voice triggers <span class="hint">(optional — for voice activation)</span></summary>
                    <div class="voice-fields">
                        <div class="field-row">
                            <div class="field">
                                <label>Phrases RU <span class="hint">(comma-separated)</span></label>
                                <textarea bind:value={form.phrases_ru} rows="2"
                                          placeholder="режим работы, запусти утро"></textarea>
                            </div>
                            <div class="field">
                                <label>Phrases EN</label>
                                <textarea bind:value={form.phrases_en} rows="2"
                                          placeholder="work mode, morning routine"></textarea>
                            </div>
                        </div>
                        <div class="field">
                            <label>Regex pattern <span class="hint">(optional)</span></label>
                            <input bind:value={form.patterns} placeholder="режим|routine" />
                        </div>
                        <div class="field">
                            <label>Response Sound <span class="hint">(optional — overrides voice pack)</span></label>
                            <select bind:value={form.response_sound}>
                                <option value="">— random from voice pack —</option>
                                {#each availableSoundFiles as f}
                                    <option value={f}>{f.split("/").pop()}</option>
                                {/each}
                            </select>
                        </div>
                    </div>
                </details>

                {#if form.formError}
                    <div class="form-error">{form.formError}</div>
                {/if}
            </div>

            <div class="modal-footer">
                <Button color="gray" radius="md" size="sm" uppercase on:click={closeModal}>
                    Cancel
                </Button>
                <Button color="lime" radius="md" size="sm" uppercase ripple
                        disabled={saving} on:click={save}>
                    {saving ? "Saving…" : (modalMode === "edit" ? "Save Changes" : "Create Script")}
                </Button>
            </div>
        </div>
    </div>
{/if}

<style lang="scss">
// ── Page ──────────────────────────────────────────────────────────────────────

.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
}
.page-title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.04em;
    text-transform: uppercase;
}
.page-sub {
    margin: 0.1rem 0 0;
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
    &.success {
        background: rgba(138,200,50,0.15);
        border: 1px solid rgba(138,200,50,0.4);
        color: #8AC832;
    }
    &.error {
        background: rgba(239,68,68,0.12);
        border: 1px solid rgba(239,68,68,0.3);
        color: #f87171;
    }
}
.empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: rgba(255,255,255,0.3);
    font-size: 0.85rem;
    p { margin: 0 0 1rem; }
}

// ── Script grid ───────────────────────────────────────────────────────────────

.script-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.9rem;
}

.script-card {
    background: rgba(20,30,35,0.7);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    transition: border-color 0.15s;
    &:hover { border-color: rgba(255,255,255,0.14); }
}

.card-header {
    padding: 0.85rem 1rem 0.65rem;
}
.card-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
}
.card-name {
    font-size: 0.9rem;
    font-weight: 700;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
}
.mode-badge {
    flex-shrink: 0;
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    border: 1px solid;
    border-radius: 4px;
    padding: 2px 6px;
}
.card-desc {
    margin: 0.35rem 0 0;
    font-size: 0.73rem;
    color: rgba(255,255,255,0.4);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

// ── Steps preview ─────────────────────────────────────────────────────────────

.steps-preview {
    padding: 0 1rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
}
.step-chip {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.72rem;
    color: rgba(255,255,255,0.55);
    background: rgba(255,255,255,0.04);
    border-radius: 5px;
    padding: 3px 8px;
    min-width: 0;
}
.step-icon { flex-shrink: 0; font-size: 0.75rem; }
.step-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
}
.step-chip.more {
    color: rgba(255,255,255,0.25);
    font-style: italic;
}
.no-steps {
    font-size: 0.72rem;
    color: rgba(255,255,255,0.2);
    font-style: italic;
}

// ── Card footer ───────────────────────────────────────────────────────────────

.card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    border-top: 1px solid rgba(255,255,255,0.05);
    background: rgba(255,255,255,0.02);
    gap: 0.5rem;
}
.step-count {
    font-size: 0.68rem;
    color: rgba(255,255,255,0.25);
    flex-shrink: 0;
}
.card-actions {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
}
.action-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    background: none;
    border: 1px solid transparent;
    border-radius: 5px;
    cursor: pointer;
    font-size: 0.7rem;
    padding: 3px 8px;
    transition: all 0.15s;
    white-space: nowrap;

    &:disabled { opacity: 0.4; cursor: default; }
}
.run-btn {
    color: #8AC832;
    border-color: rgba(138,200,50,0.3);
    background: rgba(138,200,50,0.08);
    &:hover:not(:disabled) { background: rgba(138,200,50,0.18); }
}
.edit-btn {
    color: rgba(255,255,255,0.35);
    &:hover { color: #52fefe; border-color: rgba(82,254,254,0.3); }
}
.trash-btn {
    color: rgba(255,255,255,0.25);
    &:hover { color: #f87171; border-color: rgba(248,113,113,0.3); }
    &.confirm { color: #f87171; border-color: rgba(248,113,113,0.5); background: rgba(239,68,68,0.1); }
}

// ── Modal ─────────────────────────────────────────────────────────────────────

.modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.75);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(4px);
}
.modal {
    background: #0f1a1e;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 14px;
    width: min(640px, 95vw);
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
    &::-webkit-scrollbar { width: 5px; }
    &::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.15); border-radius: 3px; }
}
.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem 0.75rem;
    border-bottom: 1px solid rgba(255,255,255,0.08);
    h3 {
        margin: 0;
        font-size: 1rem;
        font-weight: 700;
        color: #fff;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        display: flex;
        align-items: center;
    }
}
.close-btn {
    background: none; border: none;
    color: rgba(255,255,255,0.4);
    cursor: pointer; padding: 4px;
    display: flex; align-items: center;
    border-radius: 4px; transition: color 0.15s;
    &:hover { color: #fff; }
}
.modal-body {
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
}
.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid rgba(255,255,255,0.08);
}

// ── Form fields ───────────────────────────────────────────────────────────────

.field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    label {
        font-size: 0.72rem;
        font-weight: 600;
        color: rgba(255,255,255,0.6);
        text-transform: uppercase;
        letter-spacing: 0.04em;
    }
    .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.3); letter-spacing: 0; }
    input, textarea, select {
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.1);
        border-radius: 6px; color: #fff;
        font-size: 0.82rem; padding: 0.45rem 0.65rem;
        outline: none; transition: border-color 0.15s;
        resize: vertical; font-family: inherit;
        &::placeholder { color: rgba(255,255,255,0.2); }
        &:focus { border-color: rgba(82,254,254,0.4); }
    }
    select option { background: #0f1a1e; color: #fff; }
}
.field-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
}

// ── Mode tabs ─────────────────────────────────────────────────────────────────

.mode-tabs {
    display: flex;
    gap: 0.5rem;
}
.mode-tab {
    flex: 1;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: rgba(255,255,255,0.4);
    cursor: pointer;
    font-size: 0.8rem;
    font-weight: 600;
    padding: 0.6rem 0.8rem;
    text-align: left;
    transition: all 0.15s;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    &:hover { color: #fff; border-color: rgba(255,255,255,0.2); }
    &.active {
        background: rgba(82,254,254,0.08);
        border-color: rgba(82,254,254,0.35);
        color: #52fefe;
        .mode-hint { color: rgba(82,254,254,0.5); }
    }
}
.mode-hint {
    font-size: 0.65rem;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
    color: rgba(255,255,255,0.25);
}

// ── Steps builder ─────────────────────────────────────────────────────────────

.steps-section {
    background: rgba(255,255,255,0.02);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 10px;
    overflow: hidden;
}
.steps-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 0.9rem;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    background: rgba(255,255,255,0.03);
}
.section-label {
    font-size: 0.72rem;
    font-weight: 600;
    color: rgba(255,255,255,0.5);
    text-transform: uppercase;
    letter-spacing: 0.05em;
}
.add-step-btn {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    background: none;
    border: 1px solid rgba(138,200,50,0.3);
    border-radius: 5px;
    color: #8AC832;
    cursor: pointer;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 3px 10px;
    transition: all 0.15s;
    &:hover { background: rgba(138,200,50,0.1); }
}
.no-steps-hint {
    padding: 1.2rem;
    text-align: center;
    font-size: 0.75rem;
    color: rgba(255,255,255,0.2);
    font-style: italic;
}

// ── Step editor ───────────────────────────────────────────────────────────────

.step-editor {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid rgba(255,255,255,0.04);
    &:last-child { border-bottom: none; }
}
.step-controls {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    flex-shrink: 0;
    padding-top: 2px;
}
.step-num {
    font-size: 0.65rem;
    color: rgba(255,255,255,0.25);
    width: 18px;
    text-align: center;
    line-height: 1.6;
}
.step-move {
    background: none;
    border: none;
    color: rgba(255,255,255,0.2);
    cursor: pointer;
    font-size: 0.6rem;
    line-height: 1;
    padding: 1px 4px;
    transition: color 0.1s;
    &:hover:not(:disabled) { color: #fff; }
    &:disabled { opacity: 0.2; cursor: default; }
}
.step-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.4rem; }
.step-type-row {
    display: flex;
    gap: 0.4rem;
    align-items: center;
}
.step-type-select {
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 5px;
    color: #fff;
    font-size: 0.75rem;
    padding: 4px 8px;
    outline: none;
    cursor: pointer;
    flex-shrink: 0;
    option { background: #0f1a1e; }
    &:focus { border-color: rgba(82,254,254,0.3); }
}
.step-label-input {
    flex: 1;
    min-width: 0;
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.07);
    border-radius: 5px;
    color: rgba(255,255,255,0.6);
    font-size: 0.75rem;
    padding: 4px 8px;
    outline: none;
    font-family: inherit;
    &::placeholder { color: rgba(255,255,255,0.18); }
    &:focus { border-color: rgba(82,254,254,0.3); }
}
.step-fields {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
}
.field-mini {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    flex: 1;
    min-width: 120px;
    label {
        font-size: 0.65rem;
        font-weight: 600;
        color: rgba(255,255,255,0.4);
        text-transform: uppercase;
        letter-spacing: 0.04em;
    }
    .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.25); letter-spacing: 0; }
    input, select {
        background: rgba(255,255,255,0.05);
        border: 1px solid rgba(255,255,255,0.09);
        border-radius: 5px;
        color: #fff;
        font-size: 0.78rem;
        padding: 4px 8px;
        outline: none;
        font-family: inherit;
        &::placeholder { color: rgba(255,255,255,0.2); }
        &:focus { border-color: rgba(82,254,254,0.35); }
    }
    select option { background: #0f1a1e; }
}
.step-delete {
    flex-shrink: 0;
    background: none;
    border: none;
    color: rgba(255,255,255,0.2);
    cursor: pointer;
    padding: 4px;
    display: flex;
    align-items: center;
    border-radius: 4px;
    transition: color 0.1s;
    margin-top: 2px;
    &:hover { color: #f87171; }
}

// ── Voice section ─────────────────────────────────────────────────────────────

.voice-section {
    border: 1px solid rgba(255,255,255,0.06);
    border-radius: 8px;
    overflow: hidden;
    summary {
        padding: 0.6rem 0.9rem;
        cursor: pointer;
        font-size: 0.75rem;
        font-weight: 600;
        color: rgba(255,255,255,0.5);
        text-transform: uppercase;
        letter-spacing: 0.04em;
        background: rgba(255,255,255,0.03);
        user-select: none;
        .hint { text-transform: none; font-weight: 400; color: rgba(255,255,255,0.25); letter-spacing: 0; }
        &:hover { color: rgba(255,255,255,0.7); }
    }
    &[open] summary { border-bottom: 1px solid rgba(255,255,255,0.06); }
}
.voice-fields {
    padding: 0.75rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
}

// ── Form error ────────────────────────────────────────────────────────────────

.form-error {
    background: rgba(239,68,68,0.1);
    border: 1px solid rgba(239,68,68,0.3);
    border-radius: 6px;
    color: #f87171;
    font-size: 0.78rem;
    padding: 0.5rem 0.75rem;
}
</style>
