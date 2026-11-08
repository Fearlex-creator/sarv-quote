import { useState } from "react";
import { PlantIcon as Sprout } from "@phosphor-icons/react/dist/csr/Plant";
import { command } from "../lib/client";
import { writeSession } from "../lib/session";
import { useTranslation } from "../lib/i18n";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Field } from "../components/ui/shared";

export function LockScreen({ onUnlock }: { onUnlock: () => void }) {
  const t = useTranslation();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const result = await command<{
        session_token: string;
        name: string;
        role: string;
      }>("auth_login", { name, password });
      writeSession({
        token: result.session_token,
        name: result.name,
        role: result.role,
      });
      setPassword("");
      onUnlock();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="grid min-h-svh place-items-center bg-background px-4">
      <form
        onSubmit={submit}
        className="w-full max-w-sm rounded-xl border border-border bg-card p-6 shadow-sm"
      >
        <div className="mb-6 flex items-center gap-3">
          <Sprout className="size-8 text-primary" aria-hidden />
          <div>
            <h1 className="text-lg font-semibold">{t("Sign in")}</h1>
            <p className="text-sm text-muted-foreground">
              {t("Sarv keeps this shop's books on this computer.")}
            </p>
          </div>
        </div>
        <div className="flex flex-col gap-3">
          <Field label={t("Name")}>
            <Input
              autoFocus
              autoComplete="username"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </Field>
          <Field label={t("Password")}>
            <Input
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </Field>
          {error && (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}
          <Button type="submit" disabled={busy}>
            {busy ? t("Saving…") : t("Sign in")}
          </Button>
        </div>
      </form>
    </main>
  );
}
