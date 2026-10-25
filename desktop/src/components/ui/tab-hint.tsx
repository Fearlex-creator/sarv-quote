import { useTranslation } from "../../lib/i18n";
// One plain sentence under the tabs, so nobody has to learn the vocabulary.
export function TabHint({ children }: { children: string }) {
  const t = useTranslation();
  return <p className="tab-hint">{t(children)}</p>;
}
