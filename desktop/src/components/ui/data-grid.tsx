import {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
  TableRow,
  TableHead,
  TableCell,
} from "./table";
import { Input } from "./input";
import { useTranslation } from "../../lib/i18n";
import { useContext, useMemo, useState, type ReactNode } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
} from "@tanstack/react-table";
import { BooksContext } from "../../app/books";
import { ArrowDown, ArrowUp, Search } from "lucide-react";
export function DataGrid<T>({
  data,
  columns,
  id,
  empty,
  searchText,
  toolbar,
  totals,
}: {
  data: T[];
  columns: ColumnDef<T>[];
  id: (row: T) => string;
  empty: string;
  searchText?: (row: T) => string;
  toolbar?: ReactNode;
  totals?: ReactNode[];
}) {
  const t = useTranslation();
  const books = useContext(BooksContext);
  const [query, setQuery] = useState("");
  const [sorting, setSorting] = useState<SortingState>([]);
  const filtered = useMemo(
    () =>
      data.filter(
        (x) =>
          !searchText ||
          searchText(x).toLocaleLowerCase().includes(query.toLocaleLowerCase()),
      ),
    [data, query, searchText],
  );
  const identifiedColumns = useMemo(
    () =>
      columns.map((column, index) => ({
        ...column,
        id:
          column.id ??
          ("accessorKey" in column
            ? String(column.accessorKey)
            : `column-${index}`),
      })),
    [columns],
  );
  const table = useReactTable({
    data: filtered,
    columns: identifiedColumns,
    getRowId: id,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });
  return (
    <div className="data-grid">
      {(searchText || toolbar) && (
        <div className="grid-toolbar">
          {searchText && (
            <label className="search-field">
              <Search />
              <Input
                aria-label={t("Search records")}
                placeholder={t("Search records…")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
          )}
          <div className="toolbar-actions">{toolbar}</div>
          <span className="muted count">
            {t("{count} records", {
              count: new Intl.NumberFormat(
                books?.numberLocale === "fa" ? "fa-IR" : "en-US",
              ).format(filtered.length),
            })}
          </span>
        </div>
      )}
      <div className="table-scroll">
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((g) => (
              <TableRow key={g.id}>
                {g.headers.map((h) => (
                  <TableHead
                    key={h.id}
                    aria-sort={
                      h.column.getIsSorted() === "asc"
                        ? "ascending"
                        : h.column.getIsSorted() === "desc"
                          ? "descending"
                          : h.column.getCanSort()
                            ? "none"
                            : undefined
                    }
                    className={
                      (
                        h.column.columnDef.meta as
                          { numeric?: boolean } | undefined
                      )?.numeric
                        ? "numeric tabular-nums"
                        : undefined
                    }
                  >
                    {h.column.getCanSort() ? (
                      <button
                        className="sort-button"
                        onClick={h.column.getToggleSortingHandler()}
                      >
                        {flexRender(h.column.columnDef.header, h.getContext())}
                        {h.column.getIsSorted() === "asc" ? (
                          <ArrowUp />
                        ) : h.column.getIsSorted() === "desc" ? (
                          <ArrowDown />
                        ) : null}
                      </button>
                    ) : (
                      flexRender(h.column.columnDef.header, h.getContext())
                    )}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows.map((row) => (
              <TableRow key={row.id}>
                {row.getVisibleCells().map((cell) => (
                  <TableCell
                    key={cell.id}
                    className={
                      (
                        cell.column.columnDef.meta as
                          { numeric?: boolean } | undefined
                      )?.numeric
                        ? "numeric tabular-nums"
                        : undefined
                    }
                  >
                    {["classification", "normal_balance", "role"].includes(
                      "accessorKey" in cell.column.columnDef
                        ? String(cell.column.columnDef.accessorKey)
                        : "",
                    )
                      ? t(String(cell.getValue()))
                      : flexRender(
                          cell.column.columnDef.cell,
                          cell.getContext(),
                        )}
                  </TableCell>
                ))}
              </TableRow>
            ))}
            {!filtered.length && (
              <TableRow>
                <TableCell colSpan={columns.length}>
                  <div className="table-empty">
                    {query ? t("No records match your search.") : empty}
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
          {totals && (
            <TableFooter>
              <TableRow>
                {totals.map((value, i) => (
                  <TableCell key={i}>{value}</TableCell>
                ))}
              </TableRow>
            </TableFooter>
          )}
        </Table>
      </div>
    </div>
  );
}
