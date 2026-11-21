package com.suryaprofiler.ui

import com.suryaprofiler.core.ProfileLabels
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.suryaprofiler.protocol.ProfileId

/**
 * Apps Profile: search + filter, tap an app, choose its Device Profile.
 * A selection writes immediately; there is no software layer.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppsProfileScreen(vm: DynamicProfileViewModel, onBack: () -> Unit) {
    val state by vm.state.collectAsStateWithLifecycle()
    val appMap by vm.appMap.collectAsStateWithLifecycle()
    var pickerFor by remember { mutableStateOf<AppEntry?>(null) }
    var filter by remember { mutableStateOf(AppFilter.ALL) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Apps Profile") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            OutlinedTextField(
                value = state.query,
                onValueChange = vm::setQuery,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                singleLine = true,
                placeholder = { Text("Search apps…") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
            )
            Row(
                Modifier.padding(horizontal = 16.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                AppFilter.entries.forEach { f ->
                    FilterChip(
                        selected = filter == f,
                        onClick = { filter = f },
                        label = { Text(f.label) },
                        modifier = Modifier.padding(end = 8.dp),
                    )
                }
            }
            if (state.loading) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator(Modifier.size(28.dp), strokeWidth = 3.dp)
                }
            } else {
                val filtered = vm.filteredApps().filter { app ->
                    when (filter) {
                        AppFilter.ALL -> true
                        AppFilter.GAMES -> app.isGame
                        AppFilter.CONFIGURED -> appMap.containsKey(app.pkg)
                    }
                }
                val suggestions = if (state.query.isBlank() && filter != AppFilter.CONFIGURED) {
                    state.apps.filter { it.isGame && !appMap.containsKey(it.pkg) }.take(5)
                } else {
                    emptyList()
                }
                LazyColumn(
                    Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 24.dp),
                ) {
                    if (suggestions.isNotEmpty()) {
                        item(key = "suggestions") {
                            SuggestedGames(suggestions) { app ->
                                vm.setAppProfile(app.pkg, "game")
                            }
                        }
                    }
                    items(filtered, key = { it.pkg }) { app ->
                        AppRow(
                            app = app,
                            mapped = appMap[app.pkg],
                            onClick = { pickerFor = app },
                        )
                    }
                }
            }
        }
    }

    pickerFor?.let { app ->
        ModalBottomSheet(onDismissRequest = { pickerFor = null }) {
            Column(Modifier.padding(bottom = 24.dp)) {
                Column(Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) {
                    Text(app.label, style = MaterialTheme.typography.titleMedium)
                    Text(
                        app.pkg,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(
                        "Device Profile while this app is in front",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                val options = listOf<Pair<String?, String>>(null to "Default (follow base)") +
                    ProfileId.USER.map { it.wire to ProfileLabels.of(it.wire) }
                options.forEach { (value, label) ->
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .clickable {
                                vm.setAppProfile(app.pkg, value)
                                pickerFor = null
                            }
                            .padding(horizontal = 20.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = appMap[app.pkg] == value, onClick = null)
                        Spacer(Modifier.width(8.dp))
                        Text(label, style = MaterialTheme.typography.bodyLarge)
                    }
                }
            }
        }
    }
}

private enum class AppFilter(val label: String) {
    ALL("All"),
    GAMES("Games"),
    CONFIGURED("Mapped"),
}

