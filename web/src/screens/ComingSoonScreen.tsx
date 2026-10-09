import { Badge } from '@/components/ui/badge'
import { EmptyPanel } from '@/components/ui/empty-panel'
import { screenById, type ScreenId } from '@/lib/navigation'

export function ComingSoonScreen({ id }: { id: ScreenId }) {
  const meta = screenById(id)
  if (!meta) return null
  const Icon = meta.icon
  return (
    <div className="space-y-4 p-4">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="secondary">فاز {meta.phase}</Badge>
        <Badge variant="outline">وزن {meta.weight}/10</Badge>
      </div>
      <EmptyPanel
        icon={Icon}
        title={meta.title}
        description={`${meta.description} — این صفحه در نقشهٔ ساخت برنامه است و روی همان دفترکل مشترک سوار می‌شود؛ هنوز به موتور وصل نشده.`}
      />
    </div>
  )
}
