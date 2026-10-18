package network.nodex.messenger.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import network.nodex.messenger.data.ChatTheme
import network.nodex.messenger.data.ChatType
import network.nodex.messenger.data.NodeContact
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.components.NodeXBrandMark
import network.nodex.messenger.ui.theme.*

private enum class HomeTab(val label: String) { CHATS("Чаты"), CALLS("Звонки"), CONTACTS("Контакты"), SETTINGS("Настройки") }
private enum class Filter(val label: String) { ALL("Все"), DIRECT("Личные"), GROUPS("Группы"), CHANNELS("Каналы"), UNREAD("Непрочитанные") }

@Composable
fun ChatListScreen(onOpenChat: (String) -> Unit, onOpenSettings: () -> Unit) {
    val context = LocalContext.current
    val theme by NodeXRepository.currentTheme.collectAsState()
    val accentHex by NodeXRepository.customAccentHex.collectAsState()
    val accent = accentHex?.let { runCatching { Color(android.graphics.Color.parseColor(it)) }.getOrNull() }
    val palette = getThemeColors(theme, accent)
    val contacts by NodeXRepository.contacts.collectAsState()
    val network by NodeXRepository.networkStatus.collectAsState()
    var tab by remember { mutableStateOf(HomeTab.CHATS) }
    var filter by remember { mutableStateOf(Filter.ALL) }
    var searchVisible by remember { mutableStateOf(false) }
    var search by remember { mutableStateOf("") }
    var addContact by remember { mutableStateOf(false) }
    var createGroup by remember { mutableStateOf(false) }
    val chats = remember(contacts, filter, search) {
        contacts.filter { c ->
            val typeMatch = when (filter) {
                Filter.ALL -> true; Filter.DIRECT -> !c.isGroup && !c.isChannel
                Filter.GROUPS -> c.isGroup || c.chatType == ChatType.GROUP
                Filter.CHANNELS -> c.isChannel || c.chatType == ChatType.CHANNEL
                Filter.UNREAD -> c.unreadCount > 0
            }
            typeMatch && (search.isBlank() || c.name.contains(search, true) || c.userIdHex.contains(search, true))
        }
    }
    Scaffold(
        containerColor = palette.background,
        bottomBar = { BottomNavigation(tab, palette) { next -> if (next == HomeTab.SETTINGS) onOpenSettings() else tab = next } },
        floatingActionButton = {
            AnimatedVisibility(tab == HomeTab.CHATS, enter = scaleIn(spring(stiffness = 700f)) + fadeIn(), exit = scaleOut() + fadeOut()) {
                ExtendedFloatingActionButton(onClick = { addContact = true }, icon = { Icon(Icons.Default.Add, null) }, text = { Text("Новый чат") }, containerColor = palette.primary, contentColor = Color.White)
            }
        }
    ) { padding ->
        AnimatedContent(tab, modifier = Modifier.fillMaxSize().padding(padding), label = "tabs") { current ->
            when (current) {
                HomeTab.CHATS -> ChatsContent(palette, chats, network.isConnected, network.peerCount, filter, searchVisible, search, { filter = it }, { searchVisible = it }, { search = it }, { addContact = true }, { createGroup = true }, onOpenChat)
                HomeTab.CALLS -> EmptyPage("Звонки", "История защищённых вызовов появится здесь.", Icons.Default.Call, palette)
                HomeTab.CONTACTS -> EmptyPage("Контакты", "Добавьте человека по Node ID или пригласите его по ссылке.", Icons.Default.Groups, palette, { addContact = true })
                HomeTab.SETTINGS -> Unit
            }
        }
    }
    if (addContact) AddContactDialog(onDismiss = { addContact = false }, onContactAdded = onOpenChat)
    if (createGroup) CreateGroupDialog(onDismiss = { createGroup = false }, onGroupCreated = { id -> createGroup = false; onOpenChat(id) })
}

@Composable
private fun ChatsContent(palette: ThemePalette, chats: List<NodeContact>, connected: Boolean, peers: Int, filter: Filter, searchVisible: Boolean, search: String, onFilter: (Filter) -> Unit, onSearchVisibility: (Boolean) -> Unit, onSearch: (String) -> Unit, onAdd: () -> Unit, onGroup: () -> Unit, openChat: (String) -> Unit) {
    Column(Modifier.fillMaxSize()) {
        Column(Modifier.statusBarsPadding().padding(start = 20.dp, end = 20.dp, top = 14.dp)) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    NodeXBrandMark(size = 42.dp, accent = palette.primary, animated = true)
                    Spacer(Modifier.width(12.dp))
                    Column { Text("NodeX", color = palette.textPrimary, fontSize = 20.sp, fontWeight = FontWeight.Bold); NetworkStatus(connected, peers, palette) }
                }
                IconButton(onClick = { onSearchVisibility(!searchVisible) }) { Icon(Icons.Default.Search, "Поиск", tint = palette.textPrimary) }
            }
            Spacer(Modifier.height(22.dp))
            Text("Сообщения", color = palette.textPrimary, style = MaterialTheme.typography.headlineLarge, fontWeight = FontWeight.Bold, letterSpacing = (-.5).sp)
            Text("Личные и групповые беседы", color = palette.textSecondary, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(top = 4.dp))
            AnimatedVisibility(searchVisible, enter = fadeIn() + scaleIn(), exit = fadeOut() + scaleOut()) {
                TextField(value = search, onValueChange = onSearch, singleLine = true, placeholder = { Text("Имя или Node ID") }, leadingIcon = { Icon(Icons.Default.Search, null) }, modifier = Modifier.fillMaxWidth().padding(top = 16.dp).clip(RoundedCornerShape(16.dp)), colors = TextFieldDefaults.colors(focusedContainerColor = palette.surface, unfocusedContainerColor = palette.surface, focusedTextColor = palette.textPrimary, unfocusedTextColor = palette.textPrimary, focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent))
            }
        }
        LazyRow(contentPadding = PaddingValues(horizontal = 20.dp, vertical = 14.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) { items(Filter.entries) { item -> FilterChip(item, item == filter, palette) { onFilter(item) } } }
        if (chats.isEmpty()) EmptyChats(palette, onAdd, onGroup) else LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 96.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) { items(chats, key = { it.userIdHex }, contentType = { "chat" }) { ChatListItem(it, palette = palette) { openChat(it.userIdHex) } } }
    }
}

@Composable private fun NetworkStatus(connected: Boolean, peers: Int, palette: ThemePalette) {
    val color = if (connected) Color(0xFF42C487) else Color(0xFFF2AF4A)
    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(top = 2.dp)) { Box(Modifier.size(6.dp).clip(CircleShape).background(color)); Spacer(Modifier.width(6.dp)); Text(if (connected) "Сеть активна · ${nodeWord(peers)}" else "Подключение к сети", color = palette.textSecondary, fontSize = 12.sp) }
}
private fun nodeWord(count: Int): String { val last = count % 100; val word = if (last in 11..14) "узлов" else when (count % 10) { 1 -> "узел"; 2, 3, 4 -> "узла"; else -> "узлов" }; return "$count $word" }

@Composable private fun FilterChip(item: Filter, active: Boolean, palette: ThemePalette, onClick: () -> Unit) {
    val scale by animateFloatAsState(if (active) 1f else .97f, spring(stiffness = 700f), label = "filter")
    Box(Modifier.clip(RoundedCornerShape(14.dp)).background(if (active) palette.primary else palette.surface).clickable(onClick = onClick).padding(horizontal = 14.dp, vertical = 10.dp), contentAlignment = Alignment.Center) { Text(item.label, color = if (active) Color.White else palette.textSecondary, fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal, modifier = Modifier.graphicsLayer(scaleX = scale, scaleY = scale)) }
}

@Composable private fun EmptyChats(palette: ThemePalette, onAdd: () -> Unit, onGroup: () -> Unit) {
    Column(Modifier.fillMaxSize().padding(horizontal = 32.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
        Box(Modifier.size(104.dp).clip(CircleShape).background(palette.primary.copy(alpha = .12f)), contentAlignment = Alignment.Center) { Icon(Icons.Default.ChatBubbleOutline, null, tint = palette.primary, modifier = Modifier.size(42.dp)) }
        Spacer(Modifier.height(20.dp)); Text("Пока тихо", color = palette.textPrimary, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(8.dp)); Text("Добавьте контакт или создайте группу, чтобы начать переписку.", color = palette.textSecondary, textAlign = TextAlign.Center)
        Spacer(Modifier.height(24.dp)); Button(onClick = onAdd, colors = ButtonDefaults.buttonColors(containerColor = palette.primary), shape = RoundedCornerShape(16.dp)) { Icon(Icons.Default.PersonAddAlt1, null); Spacer(Modifier.width(8.dp)); Text("Добавить контакт") }
        Spacer(Modifier.height(8.dp)); OutlinedButton(onClick = onGroup, shape = RoundedCornerShape(16.dp)) { Icon(Icons.Default.Groups, null); Spacer(Modifier.width(8.dp)); Text("Создать группу") }
    }
}

@Composable private fun EmptyPage(title: String, text: String, icon: androidx.compose.ui.graphics.vector.ImageVector, palette: ThemePalette, action: (() -> Unit)? = null) {
    Column(Modifier.fillMaxSize().padding(32.dp), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) { Icon(icon, null, tint = palette.primary, modifier = Modifier.size(48.dp)); Spacer(Modifier.height(18.dp)); Text(title, color = palette.textPrimary, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold); Spacer(Modifier.height(8.dp)); Text(text, color = palette.textSecondary, textAlign = TextAlign.Center); if (action != null) { Spacer(Modifier.height(22.dp)); Button(onClick = action, colors = ButtonDefaults.buttonColors(containerColor = palette.primary)) { Text("Добавить контакт") } } }
}

@Composable private fun BottomNavigation(selected: HomeTab, palette: ThemePalette, onTab: (HomeTab) -> Unit) {
    val entries = listOf(HomeTab.CHATS to Icons.Default.ChatBubbleOutline, HomeTab.CALLS to Icons.Default.Call, HomeTab.CONTACTS to Icons.Default.Groups, HomeTab.SETTINGS to Icons.Default.Settings)
    Row(Modifier.fillMaxWidth().background(palette.surface).navigationBarsPadding().padding(horizontal = 8.dp, vertical = 8.dp), horizontalArrangement = Arrangement.SpaceAround) { entries.forEach { (item, icon) -> val active = item == selected; Column(Modifier.weight(1f).clip(RoundedCornerShape(16.dp)).clickable { onTab(item) }.padding(vertical = 7.dp), horizontalAlignment = Alignment.CenterHorizontally) { Icon(icon, item.label, tint = if (active) palette.primary else TextMuted, modifier = Modifier.size(22.dp)); Spacer(Modifier.height(3.dp)); Text(item.label, color = if (active) palette.primary else palette.textSecondary, fontSize = 11.sp, fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal) } } }
}

@Composable
fun ChatListItem(contact: NodeContact, currentUserId: String = "", palette: ThemePalette = getThemeColors(ChatTheme.GRAPHITE), onClick: () -> Unit) {
    val self = contact.isSelf || contact.userIdHex.equals(currentUserId, true)
    val initial = when { self -> "★"; contact.isGroup || contact.chatType == ChatType.GROUP -> "G"; contact.isChannel || contact.chatType == ChatType.CHANNEL -> "#"; else -> contact.name.firstOrNull()?.uppercase() ?: "?" }
    val shades = listOf(palette.primary, Color(0xFF6AA8FF), Color(0xFF9C78FA), Color(0xFF43B99B)); val shade = shades[(contact.userIdHex.hashCode() and Int.MAX_VALUE) % shades.size]
    Row(Modifier.fillMaxWidth().clip(RoundedCornerShape(22.dp)).background(palette.surface.copy(alpha = .68f)).clickable(onClick = onClick).padding(vertical = 12.dp, horizontal = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(54.dp).clip(RoundedCornerShape(19.dp)).background(Brush.linearGradient(listOf(shade, shade.copy(alpha = .55f)))), contentAlignment = Alignment.Center) { Text(initial, color = Color.White, fontSize = 20.sp, fontWeight = FontWeight.Bold) }
        Spacer(Modifier.width(13.dp)); Column(Modifier.weight(1f)) { Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) { Text(contact.name, color = palette.textPrimary, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis); Text(contact.lastMessageTime, color = TextMuted, fontSize = 12.sp) }; Spacer(Modifier.height(5.dp)); Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) { Text(if (contact.lastMessageText.isBlank()) if (self) "Личные заметки" else "Нет сообщений" else contact.lastMessageText, color = palette.textSecondary, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f)); if (contact.unreadCount > 0) { Spacer(Modifier.width(8.dp)); Box(Modifier.clip(RoundedCornerShape(10.dp)).background(Brush.horizontalGradient(listOf(NodeXBrandCyan, palette.primary))).padding(horizontal = 8.dp, vertical = 3.dp)) { Text(if (contact.unreadCount > 99) "99+" else contact.unreadCount.toString(), color = Color.White, fontSize = 11.sp, fontWeight = FontWeight.Bold) } } } }
    }
}
