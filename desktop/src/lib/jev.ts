/**
 * Optional TypeSafe "Jev" decision-model client (Beta).
 *
 * Off by default. The user enables it in Settings and provides their own
 * API key, which stays in this device's localStorage. Requests send only the
 * compact fact digests built at the call site — never the full books. The app
 * remains fully functional offline; every Jev feature degrades to a no-op.
 */
export const JEV_ENDPOINT = "https://api.typesafe.ai/v1/systemone";
export const JEV_MODEL = "jev-latest";

export type NoulQuestion = {
  type: "noul";
  instructions: string;
  criteria?: { true: string; false: string };
};
export type ChoiceQuestion = {
  type: "choice";
  instructions: string;
  criteria: Record<string, string>;
};
export type ScoreQuestion = {
  type: "score";
  instructions: string;
  criteria: string[];
};
export type Question = NoulQuestion | ChoiceQuestion | ScoreQuestion;

export type NoulAnswer = { type: "noul"; noul: number };
export type ChoiceAnswer = {
  type: "choice";
  choice: string;
  confidence: number;
  probabilities: Record<string, number>;
};
export type ScoreAnswer = {
  type: "score";
  score: number;
  confidence: number;
  legend: Record<string, string>;
  probabilities: Record<string, number>;
};
export type Answer = NoulAnswer | ChoiceAnswer | ScoreAnswer;

export type JevResponse = {
  model: string;
  answers: Record<string, Answer>;
  usage: { input_tokens: number; output_tokens: number };
};

export function jevEnabled(): boolean {
  return localStorage.getItem("sarv.ai.enabled") === "on";
}
export function jevKey(): string {
  return localStorage.getItem("sarv.ai.key") ?? "";
}
export function setJevConfig(enabled: boolean, key: string) {
  localStorage.setItem("sarv.ai.enabled", enabled && key.trim() ? "on" : "off");
  localStorage.setItem("sarv.ai.key", key.trim());
}
export function jevReady(): boolean {
  return jevEnabled() && jevKey().length > 8;
}

export class JevError extends Error {}

export async function askJev(
  state: unknown,
  questions: Record<string, Question>,
  signal?: AbortSignal,
): Promise<JevResponse> {
  const key = jevKey();
  if (!jevReady()) throw new JevError("AI assistance is off or missing a key");
  let response: Response;
  try {
    response = await fetch(JEV_ENDPOINT, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${key}`,
      },
      signal: signal ?? AbortSignal.timeout(30_000),
      body: JSON.stringify({ model: JEV_MODEL, state, questions }),
    });
  } catch (e) {
    throw new JevError(
      e instanceof Error && e.name === "TimeoutError"
        ? "The decision service did not answer in time"
        : "Could not reach the decision service",
    );
  }
  if (!response.ok) {
    if (response.status === 401 || response.status === 403)
      throw new JevError("The AI key was rejected — check it in Settings");
    if (response.status === 422)
      throw new JevError("The decision request was rejected as invalid");
    throw new JevError(`The decision service answered ${response.status}`);
  }
  return (await response.json()) as JevResponse;
}

export function asNoul(a: Answer | undefined): number | null {
  return a?.type === "noul" ? a.noul : null;
}
export function asChoice(a: Answer | undefined): ChoiceAnswer | null {
  return a?.type === "choice" ? a : null;
}
export function asScore(a: Answer | undefined): ScoreAnswer | null {
  return a?.type === "score" ? a : null;
}
