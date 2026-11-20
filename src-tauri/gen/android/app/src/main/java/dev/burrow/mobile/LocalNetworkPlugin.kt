package dev.burrow.mobile

import android.app.Activity
import app.tauri.annotation.Permission
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin

// Android 17 keeps an app off the local network (private IPv4 ranges,
// link-local and the like) until the user allows it, so a computer paired
// by its LAN address would never answer. Tailscale's addresses aren't
// local, and need nothing. Tauri's Plugin gives checkPermissions and
// requestPermissions for the alias below.
@TauriPlugin(
  permissions = [
    Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "localNetwork"),
  ],
)
class LocalNetworkPlugin(activity: Activity) : Plugin(activity)
