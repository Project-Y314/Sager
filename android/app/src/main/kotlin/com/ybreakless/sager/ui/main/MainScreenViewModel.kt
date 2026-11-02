package com.ybreakless.sager.ui.main

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.ybreakless.sager.data.SagerRepository
import com.ybreakless.sager.model.DiscoveredPc
import com.ybreakless.sager.model.MobileSyncPayload
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class MainScreenViewModel(application: Application) : AndroidViewModel(application) {

    private val repository = SagerRepository(application.applicationContext)
    val payloadState: StateFlow<MobileSyncPayload?> = repository.payloadState

    private val _isSyncing = MutableStateFlow(false)
    val isSyncing: StateFlow<Boolean> = _isSyncing.asStateFlow()

    private val _syncMessage = MutableStateFlow<String?>(null)
    val syncMessage: StateFlow<String?> = _syncMessage.asStateFlow()

    private val _activeFilter = MutableStateFlow("all") // all, unlocked, locked
    val activeFilter: StateFlow<String> = _activeFilter.asStateFlow()

    private val _selectedGameFilter = MutableStateFlow<String?>(null)
    val selectedGameFilter: StateFlow<String?> = _selectedGameFilter.asStateFlow()

    private val _discoveredPcs = MutableStateFlow(emptyList<DiscoveredPc>())
    val discoveredPcs: StateFlow<List<DiscoveredPc>> = _discoveredPcs.asStateFlow()

    private val _isDiscovering = MutableStateFlow(false)
    val isDiscovering: StateFlow<Boolean> = _isDiscovering.asStateFlow()

    fun getSavedPcHost(): String = repository.getSavedPcHost()

    fun setFilter(filter: String) {
        _activeFilter.value = filter
    }

    fun setSelectedGame(game: String?) {
        _selectedGameFilter.value = game
    }

    fun clearMessage() {
        _syncMessage.value = null
    }

    fun syncNow(customHost: String? = null) {
        viewModelScope.launch {
            _isSyncing.value = true
            val host = customHost ?: repository.getSavedPcHost()
            val result = repository.syncWithPc(host)
            _isSyncing.value = false
            result.onSuccess { payload ->
                _syncMessage.value = "Synced ${payload.unlockedAchievementsCount} trophies from ${payload.pcName}!"
            }.onFailure { err ->
                _syncMessage.value = "Sync failed: ${err.localizedMessage}. Check PC is on same Wi-Fi."
            }
        }
    }

    fun discoverPcOnWifi() {
        viewModelScope.launch {
            _isDiscovering.value = true
            try {
                val found = repository.discoverPcOnWifi()
                _discoveredPcs.value = found
                _syncMessage.value = if (found.isEmpty()) "No Sager PC found on Wi-Fi" else "Found ${found.size} Sager PC(s)"
            } catch (e: Exception) {
                _syncMessage.value = "Discovery failed: ${e.localizedMessage}"
            } finally {
                _isDiscovering.value = false
            }
        }
    }

    fun selectDiscoveredPc(pc: DiscoveredPc) {
        repository.savePcHost("${pc.ip}:${pc.port}")
        syncNow("${pc.ip}:${pc.port}")
    }

    fun pingPc(host: String) {
        viewModelScope.launch {
            val reachable = repository.pingPc(host)
            _syncMessage.value = if (reachable) "PC reachable at $host" else "PC not reachable at $host"
        }
    }

    fun importVaultJson(jsonString: String) {
        viewModelScope.launch {
            val res = repository.importVaultJson(jsonString)
            res.onSuccess { count ->
                _syncMessage.value = "Imported $count achievements into offline vault!"
            }.onFailure { err ->
                _syncMessage.value = "Import failed: ${err.localizedMessage}"
            }
        }
    }
}
