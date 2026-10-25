import {
  creationShortcuts,
  utilityShortcuts,
  shortcutKeys,
} from "../../lib/shortcuts";
import { useTranslation } from "../../lib/i18n";
import { SettingsGroup, SettingsItem } from "../ui/settings";

export function ShortcutList() {
  const t = useTranslation();
  return (
    <SettingsGroup
      title={t("Keyboard shortcuts")}
      description={t(
        "Use Ctrl + Alt on Windows/Linux, or Command + Option on macOS. Creation shortcuts work outside text fields and dialogs, using the same physical keys in Persian and English.",
      )}
      footer={t(
        "A new sales invoice starts with sale details, then opens delivery and invoice review. Keyboard shortcuts never post financial entries automatically.",
      )}
    >
      {creationShortcuts.map((s) => (
        <SettingsItem key={s.action} label={t(s.label)}>
          <kbd dir="ltr">{shortcutKeys(s.code)}</kbd>
        </SettingsItem>
      ))}
      {utilityShortcuts.map((s) => (
        <SettingsItem key={s.label} label={t(s.label)}>
          <kbd dir="ltr">{s.keys}</kbd>
        </SettingsItem>
      ))}
    </SettingsGroup>
  );
}
