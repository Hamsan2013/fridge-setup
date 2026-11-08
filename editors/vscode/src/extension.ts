import * as vscode from "vscode";
import * as cp from "child_process";
import * as fs from "fs";
import * as path from "path";

let output: vscode.OutputChannel;
let diagnostics: vscode.DiagnosticCollection;
let running: cp.ChildProcess | undefined;

export function activate(context: vscode.ExtensionContext) {
    output = vscode.window.createOutputChannel("FRIDGE");
    diagnostics = vscode.languages.createDiagnosticCollection("fridge");
    context.subscriptions.push(output, diagnostics);

    context.subscriptions.push(
        vscode.commands.registerCommand("fridge.runProgram", () => runActive()),
        vscode.commands.registerCommand("fridge.checkProgram", () => checkActive(true)),
        vscode.commands.registerCommand("fridge.stopProgram", () => stopRunning(true)),
        vscode.commands.registerCommand("fridge.formatProgram", () => soon("Format", "M5")),
        vscode.commands.registerCommand("fridge.fixProgram", () => soon("Fix Errors", "M5")),
        vscode.commands.registerCommand("fridge.buildProgram", () => soon("Build", "M5")),
        vscode.commands.registerCommand("fridge.newProject", () => soon("Create Project", "M5")),
        vscode.commands.registerCommand("fridge.openPreview", () => soon("Game Preview", "M7")),
        vscode.workspace.onDidSaveTextDocument((doc) => {
            const cfg = vscode.workspace.getConfiguration("fridge");
            if (doc.languageId === "fridge" && cfg.get<boolean>("autoCheck", true)) {
                checkFile(doc.uri.fsPath, false);
            }
        })
    );
}

export function deactivate() {
    stopRunning(false);
}

function activeFrg(): string | undefined {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !editor.document.fileName.endsWith(".frg")) {
        vscode.window.showWarningMessage("FRIDGE: open a .frg file first.");
        return undefined;
    }
    return editor.document.uri.fsPath;
}

function findInterpreter(): string | undefined {
    const configured = vscode.workspace
        .getConfiguration("fridge")
        .get<string>("interpreterPath");
    if (configured && configured.length > 0 && fs.existsSync(configured)) {
        return configured;
    }
    const names =
        process.platform === "win32" ? ["fridge.exe", "fridge"] : ["fridge"];
    const dirs = (process.env.PATH ?? "").split(
        process.platform === "win32" ? ";" : ":"
    );
    for (const dir of dirs) {
        for (const name of names) {
            const full = path.join(dir, name);
            try {
                if (fs.statSync(full).isFile()) {
                    return full;
                }
            } catch {
                /* keep searching */
            }
        }
    }
    return undefined;
}

function runActive() {
    const file = activeFrg();
    if (!file) {
        return;
    }
    const bin = findInterpreter();
    if (!bin) {
        vscode.window.showErrorMessage(
            "FRIDGE interpreter not found. Build core/ (cargo build) or set fridge.interpreterPath."
        );
        return;
    }
    stopRunning(false);
    output.show(true);
    output.appendLine(`> fridge run ${file}`);
    running = cp.spawn(bin, ["run", file], { cwd: path.dirname(file) });
    running.stdout?.on("data", (d: Buffer) => output.append(d.toString()));
    running.stderr?.on("data", (d: Buffer) => output.append(d.toString()));
    running.on("exit", (code) => {
        output.appendLine(`\n[finished with exit code ${code}]`);
        if (code !== 0) {
            checkFile(file, false);
        }
        running = undefined;
    });
}

function stopRunning(announce: boolean) {
    if (running) {
        running.kill();
        running = undefined;
        if (announce) {
            output.appendLine("[stopped]");
        }
    } else if (announce) {
        vscode.window.showInformationMessage("FRIDGE: nothing is running.");
    }
}

function checkActive(showPanel: boolean) {
    const file = activeFrg();
    if (!file) {
        return;
    }
    checkFile(file, showPanel);
}

interface FridgeDiagnostic {
    file: string;
    line: number;
    column: number;
    severity: string;
    code: string;
    message: string;
    explanation: string;
    suggestion: string;
}

function checkFile(file: string, showPanel: boolean) {
    const bin = findInterpreter();
    if (!bin) {
        return;
    }
    cp.execFile(
        bin,
        ["check", file, "--output", "json"],
        (err, stdout) => {
            const items: vscode.Diagnostic[] = [];
            try {
                const parsed = JSON.parse(stdout) as {
                    diagnostics: FridgeDiagnostic[];
                };
                for (const d of parsed.diagnostics) {
                    const range = new vscode.Range(
                        Math.max(0, d.line - 1),
                        Math.max(0, d.column - 1),
                        Math.max(0, d.line - 1),
                        Number.MAX_SAFE_INTEGER
                    );
                    const diag = new vscode.Diagnostic(
                        range,
                        `${d.message} [${d.code}]`,
                        vscode.DiagnosticSeverity.Error
                    );
                    diag.source = "fridge";
                    diag.code = d.code;
                    items.push(diag);
                }
            } catch {
                if (err && showPanel) {
                    output.show(true);
                    output.appendLine(stdout || String(err));
                }
            }
            diagnostics.set(vscode.Uri.file(file), items);
            if (showPanel) {
                if (items.length === 0) {
                    vscode.window.showInformationMessage(
                        "FRIDGE: no problems found."
                    );
                } else {
                    vscode.window.showErrorMessage(
                        `FRIDGE: ${items.length} problem(s) found — see Problems panel.`
                    );
                }
            }
        }
    );
}

function soon(feature: string, milestone: string) {
    vscode.window.showInformationMessage(
        `FRIDGE: ${feature} arrives in milestone ${milestone}. See CHANGELOG.md for status.`
    );
      }
