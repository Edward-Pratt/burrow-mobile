package dev.burrow.mobile

import android.app.Activity
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class WatchArgs {
  lateinit var onChange: Channel
}

// Tells Rust when a network comes or goes: the phone came back online, or
// moved between Wi-Fi and mobile data. A connection made over a network
// that went is dead, and waiting for the backoff or the keepalive would
// leave the app offline for up to a minute.
//
// It watches the physical networks, not the default one: with Tailscale on,
// the default network is the VPN, which stays up while the networks under
// it come and go. A request's capabilities include NOT_VPN unless removed.
@TauriPlugin
class NetworkPlugin(private val activity: Activity) : Plugin(activity) {
  private var callback: ConnectivityManager.NetworkCallback? = null

  @Command
  fun watch(invoke: Invoke) {
    val args = invoke.parseArgs(WatchArgs::class.java)
    val manager = activity.getSystemService(ConnectivityManager::class.java)
    callback?.let { manager.unregisterNetworkCallback(it) }
    // Android also calls onAvailable for each network already up when this
    // is registered. That costs the bridge one check of a live connection.
    val watcher = object : ConnectivityManager.NetworkCallback() {
      override fun onAvailable(network: Network) = args.onChange.send(JSObject())

      override fun onLost(network: Network) = args.onChange.send(JSObject())
    }
    val request = NetworkRequest.Builder()
      .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
      .build()
    manager.registerNetworkCallback(request, watcher)
    callback = watcher
    invoke.resolve()
  }
}
