import { Switch } from "../components/ui/switch";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { useTranslation } from "../lib/i18n";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { command } from "../lib/client";
import { Button } from "../components/ui/button";
import { ErrorState, Loading } from "../components/ui/shared";
import {
  SettingsGroup,
  SettingsItem,
  SettingsPage,
  SettingsRow,
} from "../components/ui/settings";
import { Status } from "../components/finance/money";

type AgentStatus = {
  enabled: boolean;
  access: "read_only" | "full";
  transport: "local";
  cli_available: boolean;
  cli_command: string;
  mcp_config: unknown;
};
async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const input = document.createElement("textarea");
    input.value = text;
    input.style.position = "fixed";
    input.style.opacity = "0";
    document.body.appendChild(input);
    input.select();
    const copied = document.execCommand("copy");
    input.remove();
    if (!copied) throw new Error("Clipboard unavailable");
  }
}
export function Agents() {
  const t = useTranslation();
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    command<AgentStatus>("agent_status").then(
      (value) => {
        if (active) setStatus(value);
      },
      (error: Error) => {
        if (active) setError(error.message);
      },
    );
    return () => {
      active = false;
    };
  }, []);
  async function configure(enabled: boolean, access: AgentStatus["access"]) {
    setBusy(true);
    setError("");
    try {
      setStatus(
        await command<AgentStatus>("agent_configure", { enabled, access }),
      );
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function copyConnection(kind: "cli" | "mcp") {
    if (!status) return;
    try {
      await copy(
        kind === "cli"
          ? status.cli_command
          : JSON.stringify(status.mcp_config, null, 2),
      );
      toast.success(t("Copied"));
    } catch (e) {
      setError((e as Error).message);
    }
  }
  return (
    <SettingsPage className="agents-panel">
      {error && <ErrorState error={error} />}
      {!status ? (
        !error && <Loading shape="form" />
      ) : (
        <>
          <SettingsGroup title={t("Agent access")}>
            <SettingsRow label={t("Agent access")}>
              <Switch
                checked={status.enabled}
                disabled={busy}
                onCheckedChange={(enabled) =>
                  void configure(enabled, status.access)
                }
              />
            </SettingsRow>
            <SettingsItem label={t("Connection")}>
              <Status kind="neutral">{t("Local")}</Status>
            </SettingsItem>
            <SettingsRow label={t("Access")}>
              <NativeSelect
                className="w-full!"
                value={status.access}
                disabled={busy}
                onChange={(e) =>
                  void configure(
                    status.enabled,
                    e.target.value as AgentStatus["access"],
                  )
                }
              >
                <NativeSelectOption value="read_only">
                  {t("Read only")}
                </NativeSelectOption>
                <NativeSelectOption value="full">
                  {t("Full access")}
                </NativeSelectOption>
              </NativeSelect>
            </SettingsRow>
            {status.enabled && !status.cli_available && (
              <div role="alert" className="px-5 py-4 text-sm text-destructive">
                {t("CLI unavailable")}
              </div>
            )}
            <SettingsItem label="CLI">
              <Button
                variant="outline"
                disabled={busy || !status.enabled || !status.cli_available}
                onClick={() => void copyConnection("cli")}
              >
                {t("Copy CLI")}
              </Button>
            </SettingsItem>
            <SettingsItem label="MCP">
              <Button
                variant="outline"
                disabled={busy || !status.enabled || !status.cli_available}
                onClick={() => void copyConnection("mcp")}
              >
                {t("Copy MCP")}
              </Button>
            </SettingsItem>
          </SettingsGroup>
          <div className="flex justify-end">
            <Button
              variant="destructive"
              disabled={busy || !status.enabled}
              onClick={() => void configure(false, status.access)}
            >
              {t("Revoke")}
            </Button>
          </div>
        </>
      )}
    </SettingsPage>
  );
}
