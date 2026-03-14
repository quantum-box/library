"use client"

import * as React from "react"
import { DayPicker } from "react-day-picker"

import { cn } from "@/lib/utils"
import { Button, buttonVariants } from "@/components/ui/button"

function Calendar({
  className,
  classNames,
  showOutsideDays = true,
  captionLayout,
  buttonVariant = "ghost",
  formatters,
  components,
  ...props
}: React.ComponentProps<typeof DayPicker> & {
  buttonVariant?: React.ComponentProps<typeof Button>["variant"]
}) {
  return (
    <DayPicker
      showOutsideDays={showOutsideDays}
      className={cn(
        "bg-background group/calendar p-3 [--cell-size:2rem] [[data-slot=card-content]_&]:bg-transparent [[data-slot=popover-content]_&]:bg-transparent",
        String.raw`rtl:**:[.rdp-button\_next>svg]:rotate-180`,
        String.raw`rtl:**:[.rdp-button\_previous>svg]:rotate-180`,
        className
      )}
      captionLayout={captionLayout}
      formatters={{
        formatMonthCaption: (date) =>
          date.toLocaleString("default", { month: "short" }),
        formatYearCaption: (date) => String(date.getFullYear()),
        ...formatters,
      }}
      classNames={{
        root: cn("w-fit"),
        months: cn("relative flex flex-col gap-4 md:flex-row"),
        month: cn("flex w-full flex-col gap-4"),
        nav: cn(
          "absolute inset-x-0 top-0 flex w-full items-center justify-between gap-1"
        ),
        nav_button_previous: cn(
          buttonVariants({ variant: buttonVariant }),
          "h-[--cell-size] w-[--cell-size] select-none p-0 aria-disabled:opacity-50"
        ),
        nav_button_next: cn(
          buttonVariants({ variant: buttonVariant }),
          "h-[--cell-size] w-[--cell-size] select-none p-0 aria-disabled:opacity-50"
        ),
        caption: cn(
          "flex h-[--cell-size] w-full items-center justify-center px-[--cell-size]"
        ),
        caption_dropdowns: cn(
          "flex h-[--cell-size] w-full items-center justify-center gap-1.5 text-sm font-medium"
        ),
        caption_label: cn(
          "select-none font-medium",
          !captionLayout
            ? "text-sm"
            : "[&>svg]:text-muted-foreground flex h-8 items-center gap-1 rounded-md pl-2 pr-1 text-sm [&>svg]:size-3.5"
        ),
        dropdown: cn("bg-popover absolute inset-0 opacity-0"),
        dropdown_month: cn("bg-popover absolute inset-0 opacity-0"),
        dropdown_year: cn("bg-popover absolute inset-0 opacity-0"),
        table: "w-full border-collapse",
        head_row: cn("flex"),
        head_cell: cn(
          "text-muted-foreground flex-1 select-none rounded-md text-[0.8rem] font-normal"
        ),
        row: cn("mt-2 flex w-full"),
        weeknumber: cn("text-muted-foreground select-none text-[0.8rem]"),
        cell: cn(
          "group/day relative aspect-square h-full w-full select-none p-0 text-center [&:has([aria-selected].day-range-end)]:rounded-r-md [&:has([aria-selected].day-range-start)]:rounded-l-md [&:has([aria-selected])]:bg-accent first:[&:has([aria-selected])]:rounded-l-md last:[&:has([aria-selected])]:rounded-r-md focus-within:relative focus-within:z-20"
        ),
        day: cn(
          buttonVariants({ variant: buttonVariant }),
          "h-[--cell-size] w-[--cell-size] p-0 font-normal aria-selected:opacity-100"
        ),
        day_range_start: "day-range-start",
        day_range_end: "day-range-end",
        day_range_middle: "aria-selected:bg-accent aria-selected:text-accent-foreground",
        day_selected:
          "bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground focus:bg-primary focus:text-primary-foreground",
        day_today: "bg-accent text-accent-foreground",
        day_outside: "text-muted-foreground aria-selected:text-muted-foreground",
        day_disabled: "text-muted-foreground opacity-50",
        day_hidden: "invisible",
        ...classNames,
      }}
      components={components}
      {...props}
    />
  )
}

export { Calendar }
