import { useState } from "react";
import { SparkleIcon as Sparkles } from "@phosphor-icons/react/dist/csr/Sparkle";
import { useTranslation } from "../../lib/i18n";
import {
  askJev,
  asChoice,
  jevReady,
  type Question,
} from "../../lib/jev";
import { Button } from "../ui/button";

/**
 * One-click Jev suggestion (Beta). Reads only; the caller decides what (if
 * anything) to do with the answer. Hidden entirely while AI is off.
 */
export function AiSuggest({
  label,
  state,
  questions,
  answerKey,
  labels,
}: {
  label: string;
  state: unknown;
  questions: Record<string, Question>;
  answerKey: string;
  labels: Record<string, string>;
}) {
  const t = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [answer, setAnswer] = useState<{ choice: string; p: number } | null>(
    null,
  );
  if (!jevReady()) return null;
  async function run() {
    setBusy(true);
    setError("");
    try {
      const r = await askJev(state, questions);
      const a = asChoice(r.answers[answerKey]);
      if (!a) throw new Error(t("The model did not return a choice"));
      setAnswer({
        choice: a.choice,
        p: Math.round((a.probabilities[a.choice] ?? 0) * 100),
      });
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="flex flex-wrap items-center gap-3 rounded-xl border border-dashed border-border bg-muted/40 px-3 py-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={busy}
        onClick={() => void run()}
      >
        <Sparkles />
        {busy ? t("Working…") : label}
      </Button>
      <span className="beta-badge !ms-0">Beta</span>
      {answer && (
        <span className="text-sm font-medium">
          {labels[answer.choice] ?? answer.choice}{" "}
          <small className="text-muted-foreground" dir="ltr">
            {answer.p}%
          </small>
        </span>
      )}
      {error && <span className="text-sm text-destructive">{error}</span>}
    </div>
  );
}
