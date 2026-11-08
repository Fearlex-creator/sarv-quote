/** Session token for the shop lock. The password is never stored here. */
const KEY = "sarv.session";

export interface ShopSession {
  token: string;
  name: string;
  role: string;
}

export function readSession(): ShopSession | null {
  try {
    const raw = sessionStorage.getItem(KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as ShopSession;
    if (!parsed.token || !parsed.role) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function writeSession(session: ShopSession | null) {
  if (!session) sessionStorage.removeItem(KEY);
  else sessionStorage.setItem(KEY, JSON.stringify(session));
}

export function sessionToken() {
  return readSession()?.token ?? "";
}
