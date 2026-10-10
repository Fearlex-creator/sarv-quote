import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "../components/ui/table";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { Preview, RecordEntry } from "../lib/types";
import { recordCommand, recordData, recordKind } from "./sales";
import { Modal } from "../components/layout/modal";
import { Button } from "../components/ui/button";
import { Field, ErrorState, Note, Loading } from "../components/ui/shared";
import {
  Money,
  Status,
  formatQuantity,
  formatDate,
} from "../components/finance/money";
export function RecordDetail({
  record,
  onClose,
  onJournal,
}: {
  record: RecordEntry;
  onClose: () => void;
  onJournal: (id: string) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const cmd = recordCommand(record);
  const data = recordData(record);
  const kind = recordKind(record);
  const [date, setDate] = useState(today);
  const [due, setDue] = useState(today);
  const [reference, setReference] = useState("");
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [error, setError] = useState("");
  const [key] = useState(uid);
  const [success, setSuccess] = useState(false);
  const sales = !!data.customer || !!data.customer_id || kind === "Fulfill";
  const defaultAction = sales
    ? kind === "Order"
      ? "fulfill"
      : kind === "Fulfill"
        ? "sale_return"
        : null
    : kind === "Order"
      ? "receive"
      : kind === "Receive"
        ? "bill"
        : kind === "Bill"
          ? "purchase_return"
          : null;
  const [action, setAction] = useState(defaultAction);
  const [options, setOptions] = useState<
    | {
        id: string;
        item_id: string;
        warehouse_id: string;
        unit_id: string;
        remaining_quantity: string;
        remaining_value: string;
      }[]
    | null
  >(null);
  const [quantities, setQuantities] = useState<Record<string, string>>({});
  const labels: Record<string, string> = {
    fulfill: "Deliver goods and issue invoice",
    receive: "Receive goods",
    bill: "Bill received goods",
    sale_return: "Restock customer return and issue credit",
    purchase_return:
      kind === "Bill"
        ? "Return billed goods and issue supplier credit"
        : "Return unbilled goods",
  };
  useEffect(() => {
    let active = true;
    if (action)
      void command<{ lines: NonNullable<typeof options> }>(
        "operation_options",
        {
          company_id: b.companyId,
          record_id: record.id,
          operation: action,
        },
      )
        .then((result) => {
          if (!active) return;
          setOptions(result.lines);
          setQuantities(
            Object.fromEntries(
              result.lines.map((l) => [
                l.id,
                formatQuantity(l.remaining_quantity),
              ]),
            ),
          );
        })
        .catch((e: Error) => {
          if (active) setError(e.message);
        });
    return () => {
      active = false;
    };
  }, [action, b.companyId, record.id]);
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    date,
    due_date: due,
    reference,
    reason: action ? labels[action] : "View record",
    record_id: record.id,
    lines: options?.map((l) => ({
      id: l.id,
      quantity: quantities[l.id] ?? "0",
    })),
    order_id: record.id,
    receipt_id: record.id,
  };
  async function run(post = false) {
    if (!action) return;
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command(`${action}_post`, {
          ...payload,
          expected_generation: preview?._generation,
        });
        await b.refresh();
        setPreview(null);
        setSuccess(true);
        toast.success(t("Goods operation posted"));
      } else setPreview(await command(`${action}_preview`, payload));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={
        cmd.reference ??
        cmd.invoice_reference ??
        cmd.return_reference ??
        t("Record #{reference}", { reference: record.sequence })
      }
      confirmation={!!preview}
      description={
        preview && action
          ? t("{operation} for {source} as {reference}. {consequence}", {
              operation: t(labels[action]),
              source:
                cmd.reference ??
                cmd.invoice_reference ??
                cmd.return_reference ??
                t("Record #{reference}", { reference: record.sequence }),
              reference,
              consequence: t(
                preview.nonfinancial
                  ? "Records the operation."
                  : "Creates a posted journal.",
              ),
            })
          : t("{section} · {kind} · recorded source #{reference}", {
              section: t(sales ? "Sales" : "Purchases"),
              kind: t(
                {
                  Order: "Order",
                  Receive: "Goods receipt",
                  Bill: "Supplier bill",
                  Fulfill: "Invoice and delivery",
                  Return: "Return and credit",
                }[kind] ?? kind,
              ),
              reference: record.sequence,
            })
      }
      wide
      footer={
        !success && action ? (
          <div className="form-actions">
            <Button
              variant="outline"
              disabled={busy || !reference || !options?.length}
              onClick={() => run()}
            >
              {t("Review financial effect")}
            </Button>
            <Button disabled={busy || !preview} onClick={() => run(true)}>
              {busy ? t("Working…") : t("Confirm and post")}
            </Button>
          </div>
        ) : undefined
      }
    >
      <div className="toolbar-actions">
        <Status kind={record.journal_id ? "positive" : "neutral"}>
          {record.journal_id ? t("Posted") : t("Recorded · nonfinancial")}
        </Status>
        {record.journal_id && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onJournal(record.journal_id!)}
          >
            {t("View linked journal")}
          </Button>
        )}
      </div>
      <div className="detail-summary">
        <div>
          <small>{t("Financial date")}</small>
          <strong>
            {formatDate(
              cmd.context.date,
              b.dateLocale,
              b.workspace.settings.date_display,
            )}
          </strong>
        </div>
        <div>
          <small>{sales ? t("Customer") : t("Supplier")}</small>
          <strong>
            {data.customer?.name ??
              data.supplier?.name ??
              b.workspace.resources?.parties.find(
                (x) => x.id === (data.customer_id ?? data.supplier_id),
              )?.name ??
              "—"}
          </strong>
        </div>
        <div>
          <small>
            {t("Total ·")}
            {b.unit === "Rial" ? "IRR" : t("toman")}
          </small>
          {data.total !== undefined ? (
            <Money value={data.total} unit={b.unit} locale={b.numberLocale} />
          ) : (
            t("Nonfinancial")
          )}
        </div>
      </div>
      {data.lines && (
        <div className="panel table-scroll">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("Product / source line")}</TableHead>
                <TableHead className="numeric">{t("Quantity")}</TableHead>
                <TableHead className="numeric">
                  {t("Net ·")}
                  {b.unit === "Rial" ? "IRR" : t("toman")}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.lines.map((l, i) => {
                const v = l as unknown as {
                  item?: { name: string };
                  item_id?: string;
                  id?: string;
                  quantity_thousandths?: string;
                  base_quantity_thousandths?: string;
                  net?: string;
                  value?: string;
                };
                return (
                  <TableRow key={v.id ?? i}>
                    <TableCell>
                      {v.item?.name ??
                        b.workspace.resources?.items.find(
                          (x) => x.id === v.item_id,
                        )?.name ??
                        t("Line {index}", { index: i + 1 })}
                    </TableCell>
                    <TableCell className="numeric">
                      {formatQuantity(
                        v.quantity_thousandths ??
                          v.base_quantity_thousandths ??
                          "0",
                        b.numberLocale,
                      )}
                    </TableCell>
                    <TableCell>
                      <Money
                        value={v.net ?? v.value ?? "0"}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
      )}
      {error && <ErrorState error={error} />}{" "}
      {success ? (
        <div className="operation-result" role="status">
          {t("Posted")}
        </div>
      ) : (
        action && (
          <>
            <h2 style={{ marginTop: 24 }}>{t(labels[action])}</h2>
            {!sales && kind === "Receive" && (
              <Field label={t("Operation")}>
                <NativeSelect
                  value={action}
                  onChange={(e) => {
                    setAction(e.target.value);
                    setOptions(null);
                    setPreview(null);
                    setError("");
                  }}
                >
                  <NativeSelectOption value="bill">
                    {t("Bill received goods")}
                  </NativeSelectOption>
                  <NativeSelectOption value="purchase_return">
                    {t("Return unbilled goods")}
                  </NativeSelectOption>
                </NativeSelect>
              </Field>
            )}
            {options === null ? (
              <Loading />
            ) : options.length === 0 ? (
              <Note>{t("No quantities remaining")}</Note>
            ) : (
              <div className="panel table-scroll" style={{ marginTop: 16 }}>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("Product / warehouse")}</TableHead>
                      <TableHead className="numeric">
                        {t("Remaining")}
                      </TableHead>
                      <TableHead className="numeric">
                        {t("Quantity to process")}
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {options.map((l) => {
                      const item = b.workspace.resources?.items.find(
                        (x) => x.id === l.item_id,
                      );
                      const warehouse = b.workspace.resources?.warehouses.find(
                        (x) => x.id === l.warehouse_id,
                      );
                      const unit = b.workspace.resources?.units.find(
                        (x) => x.id === l.unit_id,
                      );
                      return (
                        <TableRow key={l.id}>
                          <TableCell>
                            {item?.name ?? l.item_id}
                            <small className="record-unit">
                              {warehouse?.name} · {unit?.name}
                            </small>
                          </TableCell>
                          <TableCell className="numeric">
                            {formatQuantity(
                              l.remaining_quantity,
                              b.numberLocale,
                            )}
                          </TableCell>
                          <TableCell className="numeric">
                            <Input
                              aria-label={t(
                                "Quantity for {value1} in {value2}",
                                {
                                  value1: item?.name ?? l.item_id,
                                  value2: warehouse?.name ?? l.warehouse_id,
                                },
                              )}
                              inputMode="decimal"
                              value={quantities[l.id] ?? "0"}
                              onChange={(e) => {
                                setQuantities((q) => ({
                                  ...q,
                                  [l.id]: e.target.value,
                                }));
                                setPreview(null);
                              }}
                            />
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              </div>
            )}
            <div className="form-grid" style={{ marginTop: 16 }}>
              <Field label={t("New document reference")}>
                <Input
                  value={reference}
                  onChange={(e) => {
                    setReference(e.target.value);
                    setPreview(null);
                  }}
                />
              </Field>
              <Field label={t("Financial date")}>
                <Input
                  type="date"
                  value={date}
                  onChange={(e) => {
                    setDate(e.target.value);
                    setPreview(null);
                  }}
                />
              </Field>
              {(action === "bill" || action === "fulfill") && (
                <Field label={t("Due date")}>
                  <Input
                    type="date"
                    value={due}
                    min={date}
                    onChange={(e) => {
                      setDue(e.target.value);
                      setPreview(null);
                    }}
                  />
                </Field>
              )}
            </div>
            {preview && (
              <>
                <Note>
                  {t("Total:")} <Money value={preview.total} currency />
                  {preview.cost !== undefined && (
                    <>
                      {" "}
                      {t("· Cost:")} <Money value={preview.cost} currency />
                    </>
                  )}
                </Note>
                <div className="panel table-scroll">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("Account")}</TableHead>
                        <TableHead className="numeric">
                          {t("Debit · IRR")}
                        </TableHead>
                        <TableHead className="numeric">
                          {t("Credit · IRR")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {preview.journal_lines.map((l, i) => (
                        <TableRow key={i}>
                          <TableCell>
                            {
                              b.workspace.accounts.find(
                                (x) => x.id === l.account_id,
                              )?.name
                            }
                          </TableCell>
                          <TableCell>
                            <Money value={l.debit} />
                          </TableCell>
                          <TableCell>
                            <Money value={l.credit} />
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              </>
            )}
          </>
        )
      )}
    </Modal>
  );
}
