import { useEffect, useState } from "react";
import { toast } from "sonner";
import { command } from "../lib/client";
import { readSession, writeSession } from "../lib/session";
import { useTranslation } from "../lib/i18n";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Field } from "../components/ui/shared";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";

interface Status {
  configured: boolean;
  people: { id: string; name: string; role: string; disabled: boolean }[];
}

export function AccessPanel({ onReady }: { onReady?: () => void }) {
  const t = useTranslation();
  const session = readSession();
  const [status, setStatus] = useState<Status | null>(null);
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("cashier");
  const [recovery, setRecovery] = useState("");
  const [wroteItDown, setWroteItDown] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function load() {
    setStatus(await command<Status>("auth_status"));
  }
  useEffect(() => {
    void load().catch((e: Error) => setError(e.message));
  }, []);

  async function bootstrap(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const result = await command<{
        session_token: string;
        name: string;
        role: string;
        recovery_code: string;
      }>("auth_bootstrap", { name, password });
      writeSession({
        token: result.session_token,
        name: result.name,
        role: result.role,
      });
      setRecovery(result.recovery_code);
      setPassword("");
      await load();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function addUser(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      await command("auth_create_user", { name, password, role });
      setName("");
      setPassword("");
      toast.success(t("Person added. The password you typed is not saved in a report."));
      await load();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  if (!status) return <p className="text-sm text-muted-foreground">{t("Loading…")}</p>;

  if (recovery) {
    return (
      <div className="flex max-w-lg flex-col gap-3" role="dialog" aria-labelledby="recovery-title">
        <h2 id="recovery-title" className="text-base font-semibold">
          {t("Write this recovery code down. It will not be shown again.")}
        </h2>
        <p className="text-sm text-muted-foreground">
          {t("There is no email reset. The code stays out of the books and out of logs.")}
        </p>
        <p className="rounded-md bg-muted px-3 py-2 font-mono text-lg tracking-widest" dir="ltr">
          {recovery}
        </p>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={wroteItDown}
            onChange={(e) => setWroteItDown(e.target.checked)}
          />
          {t("I wrote the recovery code down")}
        </label>
        <Button
          type="button"
          disabled={!wroteItDown}
          onClick={() => {
            setRecovery("");
            toast.success(t("Shop lock is on"));
            onReady?.();
          }}
        >
          {t("Continue")}
        </Button>
      </div>
    );
  }

  if (!status.configured) {
    return (
      <form onSubmit={bootstrap} className="flex max-w-md flex-col gap-3">
        <p className="text-sm text-muted-foreground">
          {t("This shop is not password protected. Anyone who can open Sarv can change the books.")}
        </p>
        <Field label={t("Owner name")}>
          <Input value={name} onChange={(e) => setName(e.target.value)} required autoComplete="username" />
        </Field>
        <Field label={t("Password")}>
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            minLength={10}
            autoComplete="new-password"
          />
        </Field>
        {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
        <Button type="submit" disabled={busy}>{t("Set a password")}</Button>
      </form>
    );
  }

  const owner = session?.role === "owner" || session?.role === "admin";
  return (
    <div className="flex max-w-lg flex-col gap-4">
      <p className="text-sm text-muted-foreground">
        {t("Signed in as {name} ({role}).", {
          name: session?.name ?? "",
          role: session?.role ?? "",
        })}
      </p>
      <ul className="flex flex-col gap-1 text-sm">
        {status.people.map((person) => (
          <li key={person.id} className="flex justify-between gap-3">
            <span>{person.name}</span>
            <span className="text-muted-foreground" dir="ltr">
              {person.disabled ? t("Disabled") : person.role}
            </span>
          </li>
        ))}
      </ul>
      {owner && (
        <form onSubmit={addUser} className="flex flex-col gap-3 border-t border-border pt-4">
          <h2 className="text-sm font-medium">{t("Add person")}</h2>
          <Field label={t("Name")}>
            <Input value={name} onChange={(e) => setName(e.target.value)} required />
          </Field>
          <Field label={t("Password")}>
            <Input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              minLength={10}
              autoComplete="new-password"
            />
          </Field>
          <Field label={t("Role")}>
            <NativeSelect value={role} onChange={(e) => setRole(e.target.value)}>
              <NativeSelectOption value="cashier">{t("Cashier")}</NativeSelectOption>
              <NativeSelectOption value="accountant">{t("Accountant")}</NativeSelectOption>
              <NativeSelectOption value="readonly">{t("Read only")}</NativeSelectOption>
              <NativeSelectOption value="admin">{t("Admin")}</NativeSelectOption>
            </NativeSelect>
          </Field>
          {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
          <Button type="submit" disabled={busy}>{t("Add person")}</Button>
        </form>
      )}
    </div>
  );
}
