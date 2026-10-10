import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from "../components/ui/command";
import { Modal } from "../components/layout/modal";
import { navigation } from "../app/navigation";
import { useTranslation } from "../lib/i18n";
import type { Page } from "../lib/types";

export function CommandMenu({
  onClose,
  onNavigate,
  onSetup,
  onJournal,
  onToggleSidebar,
  canPost,
}: {
  onClose: () => void;
  onNavigate: (page: Page) => void;
  onSetup: () => void;
  onJournal: () => void;
  onToggleSidebar: () => void;
  canPost: boolean;
}) {
  const t = useTranslation();
  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={t("Command menu")}
      description={t(
        "Search pages and actions. Use arrow keys and Enter to select.",
      )}
    >
      <Command label={t("Search commands")}>
        <CommandInput
          data-autofocus="true"
          placeholder={t("Type a command…")}
          aria-label={t("Search commands")}
        />
        <CommandList aria-label={t("Search results")}>
          <CommandEmpty>{t("No matching commands.")}</CommandEmpty>
          <CommandGroup heading={t("Navigate")}>
            {navigation.map((n) => (
              <CommandItem
                key={n.page}
                value={t(n.label)}
                keywords={[n.label]}
                onSelect={() => onNavigate(n.page)}
              >
                <n.icon />
                {t(n.label)}
              </CommandItem>
            ))}
          </CommandGroup>
          <CommandGroup heading={t("Actions")}>
            <CommandItem onSelect={onSetup}>{t("Set up company")}</CommandItem>
            <CommandItem disabled={!canPost} onSelect={onJournal}>
              {t("New journal")}
            </CommandItem>
            <CommandItem onSelect={onToggleSidebar}>
              {t("Toggle sidebar")}
              <kbd>F9</kbd>
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </Modal>
  );
}
