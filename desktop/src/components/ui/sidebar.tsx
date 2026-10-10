import { createContext, useContext, type ReactNode } from "react";
import { cn } from "../../lib/utils";
const Context = createContext({ collapsed: false });
export function SidebarProvider({
  collapsed,
  children,
}: {
  collapsed: boolean;
  children: ReactNode;
}) {
  return <Context.Provider value={{ collapsed }}>{children}</Context.Provider>;
}
export function Sidebar({ children }: { children: ReactNode }) {
  const { collapsed } = useContext(Context);
  return (
    <aside className={cn("sidebar", collapsed && "sidebar-collapsed")}>
      {children}
    </aside>
  );
}
export function SidebarHeader({ children }: { children: ReactNode }) {
  return <div className="sidebar-header">{children}</div>;
}
export function SidebarContent({ children }: { children: ReactNode }) {
  return <div className="sidebar-content">{children}</div>;
}
export function SidebarFooter({ children }: { children: ReactNode }) {
  return <div className="sidebar-footer">{children}</div>;
}
