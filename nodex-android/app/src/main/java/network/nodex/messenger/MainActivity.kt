package network.nodex.messenger

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.*
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.IntOffset
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.screens.AuthScreen
import network.nodex.messenger.ui.screens.ChatDetailScreen
import network.nodex.messenger.ui.screens.ChatListScreen
import network.nodex.messenger.ui.screens.SettingsScreen
import network.nodex.messenger.ui.theme.NodeXTheme
import network.nodex.messenger.ui.theme.getThemeColors

import network.nodex.messenger.ui.screens.CallScreen

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import network.nodex.messenger.ui.theme.TextMuted
import network.nodex.messenger.ui.theme.TextPrimary
import network.nodex.messenger.ui.theme.TextSecondary

sealed interface Screen {
    data object Auth : Screen
    data object ChatList : Screen
    data class ChatDetail(val contactId: String) : Screen
    data class Call(val peerId: String, val peerName: String, val isIncoming: Boolean) : Screen
    data object Settings : Screen
}

class MainActivity : ComponentActivity() {

    private var currentScreen by mutableStateOf<Screen>(Screen.ChatList)

    override fun onStop() {
        // The current call transport is owned by the activity process and has
        // no foreground call service. End calls while the app is backgrounded
        // so the remote side receives a signed Hangup instead of ringing forever.
        if (NodeXRepository.activeCall.value != null) {
            NodeXRepository.endCall()
        }
        super.onStop()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        NodeXRepository.initialize(applicationContext)

        val isOnboardedInitial = NodeXRepository.isOnboarded.value
        currentScreen = if (isOnboardedInitial) Screen.ChatList else Screen.Auth

        intent?.getStringExtra("open_chat_id")?.let { chatId ->
            if (chatId.isNotBlank()) {
                currentScreen = Screen.ChatDetail(chatId)
            }
        }

        handleDeepLink(intent)

        setContent {
            val currentTheme by NodeXRepository.currentTheme.collectAsState()
            val customAccentHex by NodeXRepository.customAccentHex.collectAsState()
            val customAccent = customAccentHex?.let {
                try {
                    androidx.compose.ui.graphics.Color(android.graphics.Color.parseColor(it))
                } catch (e: Exception) {
                    null
                }
            }
            val isOnboarded by NodeXRepository.isOnboarded.collectAsState()
            val activeCall by NodeXRepository.activeCall.collectAsState()
            val topPushNotification by NodeXRepository.topPushNotification.collectAsState()

            LaunchedEffect(isOnboarded) {
                if (!isOnboarded) {
                    currentScreen = Screen.Auth
                } else if (currentScreen is Screen.Auth) {
                    currentScreen = Screen.ChatList
                }
            }

            LaunchedEffect(activeCall) {
                val call = activeCall
                if (call != null && call.isIncoming && currentScreen !is Screen.Call) {
                    currentScreen = Screen.Call(call.peerId, call.peerName, isIncoming = true)
                }
            }

            NodeXTheme(theme = currentTheme, customAccent = customAccent) {
                val palette = getThemeColors(currentTheme, customAccent)
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = palette.background
                ) {
                    Box(modifier = Modifier.fillMaxSize()) {
                        NavigationHost(
                            currentScreen = currentScreen,
                            onAuthenticated = { currentScreen = Screen.ChatList },
                            onOpenChat = { id -> currentScreen = Screen.ChatDetail(id) },
                            onOpenSettings = { currentScreen = Screen.Settings },
                            onStartCall = { peerId, peerName, isIncoming ->
                                currentScreen = Screen.Call(peerId, peerName, isIncoming)
                            },
                            onBack = { currentScreen = Screen.ChatList },
                            onLogout = { currentScreen = Screen.Auth }
                        )

                        // In-App Floating Top Push Notification Banner
                        AnimatedVisibility(
                            visible = topPushNotification != null,
                            enter = slideInVertically(initialOffsetY = { -it }) + fadeIn(),
                            exit = slideOutVertically(targetOffsetY = { -it }) + fadeOut(),
                            modifier = Modifier
                                .align(Alignment.TopCenter)
                                .padding(top = 46.dp, start = 16.dp, end = 16.dp)
                                .fillMaxWidth()
                        ) {
                            val notif = topPushNotification
                            if (notif != null) {
                                LaunchedEffect(notif.id) {
                                    delay(4000)
                                    NodeXRepository.dismissTopPushNotification()
                                }

                                Card(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clickable {
                                            val sender = notif.senderId
                                            NodeXRepository.dismissTopPushNotification()
                                            currentScreen = Screen.ChatDetail(sender)
                                        },
                                    shape = RoundedCornerShape(16.dp),
                                    colors = CardDefaults.cardColors(containerColor = palette.surface),
                                    border = androidx.compose.foundation.BorderStroke(1.2.dp, palette.primary)
                                ) {
                                    Row(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .padding(horizontal = 14.dp, vertical = 12.dp),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Box(
                                            modifier = Modifier
                                                .size(40.dp)
                                                .clip(CircleShape)
                                                .background(palette.primary.copy(alpha = 0.2f)),
                                            contentAlignment = Alignment.Center
                                        ) {
                                            Text(
                                                text = notif.senderName.take(1).uppercase(),
                                                color = palette.primary,
                                                fontWeight = FontWeight.Bold,
                                                fontSize = 18.sp
                                            )
                                        }
                                        Spacer(modifier = Modifier.width(12.dp))
                                        Column(modifier = Modifier.weight(1f)) {
                                            Row(verticalAlignment = Alignment.CenterVertically) {
                                                Text(
                                                    text = notif.senderName,
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    fontWeight = FontWeight.Bold,
                                                    color = TextPrimary
                                                )
                                                Spacer(modifier = Modifier.width(6.dp))
                                                Text(
                                                    text = "• сейчас",
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = palette.primary
                                                )
                                            }
                                            Spacer(modifier = Modifier.height(2.dp))
                                            Text(
                                                text = notif.text,
                                                style = MaterialTheme.typography.bodySmall,
                                                color = TextSecondary,
                                                maxLines = 1,
                                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis
                                            )
                                        }
                                        IconButton(
                                            onClick = { NodeXRepository.dismissTopPushNotification() },
                                            modifier = Modifier.size(28.dp)
                                        ) {
                                            Icon(
                                                imageVector = Icons.Default.Close,
                                                contentDescription = "Закрыть",
                                                tint = TextMuted,
                                                modifier = Modifier.size(16.dp)
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        intent.getStringExtra("open_chat_id")?.let { chatId ->
            if (chatId.isNotBlank()) {
                currentScreen = Screen.ChatDetail(chatId)
            }
        }
        handleDeepLink(intent)
    }

    private fun handleDeepLink(intent: Intent?) {
        val uri = intent?.data ?: return
        if (uri.scheme == "nodex" && uri.host == "invite") {
            val peerId = uri.lastPathSegment ?: uri.path?.removePrefix("/") ?: return
            if (peerId.isNotBlank()) {
                NodeXRepository.addContact(peerId, "Контакт из ссылки")
                if (NodeXRepository.isOnboarded.value) {
                    currentScreen = Screen.ChatDetail(peerId)
                }
            }
        }
    }
}

@Composable
fun NavigationHost(
    currentScreen: Screen,
    onAuthenticated: () -> Unit,
    onOpenChat: (String) -> Unit,
    onOpenSettings: () -> Unit,
    onStartCall: (peerId: String, peerName: String, isIncoming: Boolean) -> Unit,
    onBack: () -> Unit,
    onLogout: () -> Unit
) {
    BackHandler(enabled = currentScreen !is Screen.ChatList && currentScreen !is Screen.Auth) {
        if (currentScreen is Screen.Call) {
            NodeXRepository.endCall()
        }
        onBack()
    }

    AnimatedContent(
        targetState = currentScreen,
        transitionSpec = {
            val animSpec = tween<Float>(durationMillis = 280, easing = FastOutSlowInEasing)
            val slideSpec = tween<IntOffset>(durationMillis = 380, easing = FastOutSlowInEasing)

            if (initialState is Screen.Auth || targetState !is Screen.ChatList) {
                // Forward navigation (push)
                (slideInHorizontally(slideSpec) { width -> width } + fadeIn(animSpec))
                    .togetherWith(slideOutHorizontally(slideSpec) { width -> -width / 3 } + fadeOut(animSpec))
            } else {
                // Backward navigation (pop)
                (slideInHorizontally(slideSpec) { width -> -width / 3 } + fadeIn(animSpec))
                    .togetherWith(slideOutHorizontally(slideSpec) { width -> width } + fadeOut(animSpec))
            }
        },
        label = "AppScreenTransition"
    ) { screen ->
        when (screen) {
            is Screen.Auth -> {
                AuthScreen(
                    onAuthenticated = onAuthenticated
                )
            }
            is Screen.ChatList -> {
                ChatListScreen(
                    onOpenChat = onOpenChat,
                    onOpenSettings = onOpenSettings
                )
            }
            is Screen.ChatDetail -> {
                ChatDetailScreen(
                    contactId = screen.contactId,
                    onBack = onBack,
                    onStartCall = { peerId, peerName ->
                        onStartCall(peerId, peerName, false)
                    }
                )
            }
            is Screen.Call -> {
                CallScreen(
                    peerId = screen.peerId,
                    peerName = screen.peerName,
                    isIncoming = screen.isIncoming,
                    onBack = {
                        NodeXRepository.endCall()
                        onBack()
                    }
                )
            }
            is Screen.Settings -> {
                SettingsScreen(
                    onBack = onBack,
                    onLogout = onLogout
                )
            }
        }
    }
}
