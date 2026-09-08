import {
  AlignLeft,
  CalendarDays,
  CheckSquare,
  Code2,
  FileText,
  Hash,
  Image as ImageIcon,
  Key,
  Link2,
  List,
  MapPin,
  Tag,
  Type,
} from 'lucide-react'

const iconByType: Record<string, typeof Type> = {
  String: Type,
  Integer: Hash,
  Html: Code2,
  Markdown: FileText,
  RichText: AlignLeft,
  Relation: Link2,
  Select: Tag,
  MultiSelect: List,
  Id: Key,
  Location: MapPin,
  Date: CalendarDays,
  Image: ImageIcon,
  Boolean: CheckSquare,
}

/** The glyph a column header wears, so a type is readable without a menu. */
export function PropertyTypeIcon({ typ, className }: { typ: string; className?: string }) {
  const Icon = iconByType[typ] ?? Type
  return <Icon className={className ?? 'size-3.5'} aria-hidden="true" />
}
