"use client"

import { useTheme } from "next-themes"
import { Toaster as Sonner, type ToasterProps } from "sonner"
import { CircleCheckIcon, InfoIcon, TriangleAlertIcon, OctagonXIcon, Loader2Icon } from "lucide-react"

const Toaster = ({ ...props }: ToasterProps) => {
  const { theme = "system" } = useTheme()

  return (
    <Sonner
      theme={theme as ToasterProps["theme"]}
      className="toaster group"
      icons={{
        success: (
          <CircleCheckIcon className="size-4" />
        ),
        info: (
          <InfoIcon className="size-4" />
        ),
        warning: (
          <TriangleAlertIcon className="size-4" />
        ),
        error: (
          <OctagonXIcon className="size-4" />
        ),
        loading: (
          <Loader2Icon className="size-4 animate-spin" />
        ),
      }}
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
          "--border-radius": "var(--radius)",
        } as React.CSSProperties
      }
      // Changed from shadcn's: an action ("View order") is Ink Blue like every other
      // action, not ink. "!" beats sonner's own, more specific styles.
      toastOptions={{
        classNames: {
          toast: "cn-toast",
          actionButton: "bg-primary! text-primary-foreground!",
        },
      }}
      // Changed from shadcn's: toasts rise above the trade page's Buy/Sell bar while it
      // shows (its height is in --toast-lift, see MobileTradeBar). 24px and 16px are
      // sonner's own distances from the bottom; 16px applies below 600px.
      offset={{ bottom: "calc(var(--toast-lift, 0px) + 24px)" }}
      mobileOffset={{ bottom: "calc(var(--toast-lift, 0px) + 16px)" }}
      {...props}
    />
  )
}

export { Toaster }
