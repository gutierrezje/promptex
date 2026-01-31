import { Link, useRouteContext, useRouterState } from '@tanstack/react-router'
import { useServerFn } from '@tanstack/react-start'
import {
  LayoutDashboard,
  AlertCircle,
  GitCommit,
  GitPullRequest,
  Tag,
  Compass,
  Settings,
  BookMarked,
  LogOut,
} from 'lucide-react'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar'
import { logout } from '@/server/functions/auth.fns'


const mainNav = [
  { label: 'Dashboard', href: '/dashboard', icon: LayoutDashboard },
  { label: 'Discover', href: '/discover', icon: Compass },
  { label: 'Issues', href: '/issues', icon: AlertCircle },
  { label: 'Commits', href: '/commits', icon: GitCommit },
  { label: 'Pull Requests', href: '/pulls', icon: GitPullRequest },
  { label: 'Releases', href: '/releases', icon: Tag },
] as const

const manageNav = [
  { label: 'Subscriptions', href: '/subscriptions', icon: BookMarked },
  { label: 'Settings', href: '/settings', icon: Settings },
] as const

export function AppSidebar() {
  const routerState = useRouterState()
  const currentPath = routerState.location.pathname
  const { user } = useRouteContext({ from: '/_authed' })
  const handleLogout = useServerFn(logout)

  if (!user) return null

  return (
    <Sidebar>
      <SidebarHeader>
        <div className="flex items-center gap-2 px-2 py-1">
          <div className="flex h-8 w-8 items-center justify-center rounded-md bg-emerald-600 text-sm font-bold text-white">
            IS
          </div>
          <span className="text-lg font-semibold tracking-tight">
            Issuance
          </span>
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Feed</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {mainNav.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton
                    asChild
                    isActive={currentPath === item.href}
                  >
                    <Link to={item.href}>
                      <item.icon />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Manage</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {manageNav.map((item) => (
                <SidebarMenuItem key={item.href}>
                  <SidebarMenuButton
                    asChild
                    isActive={currentPath === item.href}
                  >
                    <Link to={item.href}>
                      <item.icon />
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter>
        <div className="flex items-center justify-between px-2 py-1">
          <div className="flex items-center gap-2 min-w-0">
            {user.avatarUrl ? (
              <img
                src={user.avatarUrl}
                alt={user.username}
                className="h-7 w-7 rounded-full shrink-0"
              />
            ) : (
              <div className="flex h-7 w-7 items-center justify-center rounded-full bg-slate-700 text-xs font-medium shrink-0">
                {user.username[0]?.toUpperCase()}
              </div>
            )}
            <span className="text-sm font-medium truncate">
              {user.displayName ?? user.username}
            </span>
          </div>
          <button
            onClick={() => handleLogout().then(() => window.location.assign('/login'))}
            className="text-muted-foreground hover:text-foreground transition-colors p-1 rounded-md hover:bg-slate-800"
            title="Sign out"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>
      </SidebarFooter>
    </Sidebar>
  )
}
