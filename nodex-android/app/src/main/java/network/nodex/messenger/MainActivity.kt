package network.nodex.messenger

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.screens.ChatDetailScreen
import network.nodex.messenger.ui.screens.ChatListScreen
import network.nodex.messenger.ui.theme.NodeXBackground
import network.nodex.messenger.ui.theme.NodeXTheme

class MainActivity : ComponentActivity() {

    private var activeChatId by mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        handleDeepLink(intent)

        setContent {
            NodeXTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = NodeXBackground
                ) {
                    MainNavigation(
                        currentChatId = activeChatId,
                        onOpenChat = { id -> activeChatId = id },
                        onBack = { activeChatId = null }
                    )
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleDeepLink(intent)
    }

    private fun handleDeepLink(intent: Intent?) {
        val uri = intent?.data ?: return
        if (uri.scheme == "nodex" && uri.host == "invite") {
            val peerId = uri.lastPathSegment ?: uri.path?.removePrefix("/") ?: return
            if (peerId.isNotBlank()) {
                NodeXRepository.addContact(peerId, "Контакт из ссылки")
                activeChatId = peerId.take(16)
            }
        }
    }
}

@Composable
fun MainNavigation(
    currentChatId: String?,
    onOpenChat: (String) -> Unit,
    onBack: () -> Unit
) {
    BackHandler(enabled = currentChatId != null) {
        onBack()
    }

    AnimatedContent(
        targetState = currentChatId,
        transitionSpec = {
            if (targetState != null) {
                (slideInHorizontally { width -> width } + fadeIn()).togetherWith(
                    slideOutHorizontally { width -> -width / 3 } + fadeOut()
                )
            } else {
                (slideInHorizontally { width -> -width / 3 } + fadeIn()).togetherWith(
                    slideOutHorizontally { width -> width } + fadeOut()
                )
            }
        },
        label = "ChatNavigationTransition"
    ) { chatId ->
        if (chatId == null) {
            ChatListScreen(onOpenChat = onOpenChat)
        } else {
            ChatDetailScreen(
                contactId = chatId,
                onBack = onBack
            )
        }
    }
}
