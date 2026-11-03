import { Alert, AlertDescription, AlertTitle } from "../ui/alert";
import { useTranslation } from "../../lib/i18n";

/**
 * Show this wherever a legal value (tax rate, salary-tax bracket, insurance rate) is displayed
 * or used. The values are unreviewed until the owner signs them off; see `iran::law`.
 */
export function LawNotice() {
  const t = useTranslation();
  return (
    <Alert data-testid="law-not-reviewed">
      <AlertTitle>{t("Legal values not yet reviewed")}</AlertTitle>
      <AlertDescription>
        {t(
          "Not yet reviewed. Do not use these values for real invoices, payslips or tax filings.",
        )}
      </AlertDescription>
    </Alert>
  );
}
