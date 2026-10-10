import {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
  TableRow,
  TableHead,
  TableCell,
} from "./table";
import { InputGroup, InputGroupInput, InputGroupAddon } from "./input-group";
import { Button } from "./button";
import { useTranslation } from "../../lib/i18n";
import {
  useContext,
  useDeferredValue,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
} from "@tanstack/react-table";
import { BooksContext } from "../../app/books";
import { ArrowDownIcon as ArrowDown } from "@phosphor-icons/react/dist/csr/ArrowDown";
import { ArrowUpIcon as ArrowUp } from "@phosphor-icons/react/dist/csr/ArrowUp";
import { MagnifyingGlassIcon as Search } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
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
  const deferredQuery = useDeferredValue(query);
  const searchIndex = useMemo(
    () =>
      searchText
        ? data.map((row) => searchText(row).toLocaleLowerCase())
        : null,
    [data, searchText],
  );
  const filtered = useMemo(() => {
    if (!deferredQuery || !searchIndex) return data;
    const term = deferredQuery.toLocaleLowerCase();
    return data.filter((_, index) => searchIndex[index].includes(term));
  }, [data, deferredQuery, searchIndex]);
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
            <InputGroup className="w-56">
              <InputGroupAddon>
                <Search />
              </InputGroupAddon>
              <InputGroupInput
                aria-label={t("Search records")}
                placeholder={t("Search records…")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </InputGroup>
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
      <div className="table-scroll" aria-busy={query !== deferredQuery}>
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
                      <Button
                        variant="ghost"
                        size="sm"
                        className="sort-button"
                        onClick={h.column.getToggleSortingHandler()}
                      >
                        {flexRender(h.column.columnDef.header, h.getContext())}
                        {h.column.getIsSorted() === "asc" ? (
                          <ArrowUp />
                        ) : h.column.getIsSorted() === "desc" ? (
                          <ArrowDown />
                        ) : null}
                      </Button>
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
