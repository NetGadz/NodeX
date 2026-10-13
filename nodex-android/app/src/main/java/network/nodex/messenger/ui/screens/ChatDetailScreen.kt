package network.nodex.messenger.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.DoneAll
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.PhoneInTalk
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import network.nodex.messenger.data.ChatMessage
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.theme.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatDetailScreen(
    contactId: String,
    onBack: () -> Unit
) {
    val contacts by NodeXRepository.contacts.collectAsState()
    val allMessages by NodeXRepository.messages.collectAsState()
    val contact = contacts.firstOrNull { it.userIdHex == contactId }
    val messages = allMessages[contactId] ?: emptyList()

    var inputText by remember { mutableStateOf("") }

    Scaffold(
        containerColor = NodeXBackground,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(containerColor = NodeXSurface),
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back",
                            tint = TextPrimary
                        )
                    }
                },
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(38.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .background(NodeXSurfaceLight),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = contact?.name?.take(1) ?: "P",
                                fontWeight = FontWeight.Bold,
                                color = NodeXCyan
                            )
                        }
                        Spacer(modifier = Modifier.width(10.dp))
                        Column {
                            Text(
                                text = contact?.name ?: "Peer",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = TextPrimary
                            )
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(
                                    imageVector = Icons.Default.Lock,
                                    contentDescription = "E2EE",
                                    tint = NodeXEmerald,
                                    modifier = Modifier.size(11.dp)
                                )
                                Spacer(modifier = Modifier.width(4.dp))
                                Text(
                                    text = "ChaCha20 E2EE • P2P Direct",
                                    fontSize = 11.sp,
                                    color = NodeXEmerald
                                )
                            }
                        }
                    }
                },
                actions = {
                    IconButton(onClick = {}) {
                        Icon(
                            imageVector = Icons.Default.PhoneInTalk,
                            contentDescription = "Audio Call",
                            tint = NodeXCyan
                        )
                    }
                }
            )
        },
        bottomBar = {
            Surface(
                color = NodeXSurface,
                tonalElevation = 8.dp
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 10.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    IconButton(onClick = {}) {
                        Icon(
                            imageVector = Icons.Default.AttachFile,
                            contentDescription = "Attach",
                            tint = TextSecondary
                        )
                    }
                    OutlinedTextField(
                        value = inputText,
                        onValueChange = { inputText = it },
                        placeholder = { Text("Зашифрованное P2P сообщение...") },
                        modifier = Modifier
                            .weight(1f)
                            .padding(horizontal = 4.dp),
                        shape = RoundedCornerShape(24.dp),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedContainerColor = NodeXBackground,
                            unfocusedContainerColor = NodeXBackground,
                            focusedBorderColor = NodeXCyan,
                            unfocusedBorderColor = NodeXBorder,
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary
                        ),
                        maxLines = 4
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    IconButton(
                        onClick = {
                            if (inputText.isNotBlank()) {
                                NodeXRepository.sendMessage(contactId, inputText.trim())
                                inputText = ""
                            }
                        },
                        modifier = Modifier
                            .size(44.dp)
                            .clip(CircleShape)
                            .background(Brush.linearGradient(listOf(NodeXCyan, NodeXBlue)))
                    ) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.Send,
                            contentDescription = "Send",
                            tint = Color.Black,
                            modifier = Modifier.size(20.dp)
                        )
                    }
                }
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(horizontal = 14.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            items(messages, key = { it.id }) { msg ->
                MessageBubble(message = msg)
            }
        }
    }
}

@Composable
fun MessageBubble(message: ChatMessage) {
    val isOut = !message.isIncoming
    val alignment = if (isOut) Alignment.End else Alignment.Start

    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = alignment
    ) {
        Box(
            modifier = Modifier
                .widthIn(max = 290.dp)
                .clip(
                    RoundedCornerShape(
                        topStart = 18.dp,
                        topEnd = 18.dp,
                        bottomStart = if (isOut) 18.dp else 4.dp,
                        bottomEnd = if (isOut) 4.dp else 18.dp
                    )
                )
                .background(
                    if (isOut) Brush.linearGradient(listOf(NodeXBubbleOut, Color(0xFF0369A1)))
                    else Brush.linearGradient(listOf(NodeXBubbleIn, NodeXBubbleIn))
                )
                .padding(horizontal = 14.dp, vertical = 10.dp)
        ) {
            if (message.isVoice) {
                // Voice Waveform Player
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(Color.White.copy(alpha = 0.2f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.PlayArrow,
                            contentDescription = "Play",
                            tint = Color.White
                        )
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Row(
                        modifier = Modifier.height(24.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(3.dp)
                    ) {
                        val heights = listOf(10.dp, 16.dp, 24.dp, 12.dp, 22.dp, 18.dp, 8.dp, 20.dp, 24.dp, 14.dp, 16.dp)
                        heights.forEachIndexed { i, h ->
                            Box(
                                modifier = Modifier
                                    .width(3.5.dp)
                                    .height(h)
                                    .clip(RoundedCornerShape(2.dp))
                                    .background(if (i < 5) NodeXCyan else Color.White.copy(alpha = 0.4f))
                            )
                        }
                    }
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "0:${message.voiceDurationSecs}",
                        fontSize = 11.sp,
                        color = Color.White.copy(alpha = 0.8f)
                    )
                }
            } else {
                // Text Message
                Column {
                    Text(
                        text = message.text,
                        color = Color.White,
                        style = MaterialTheme.typography.bodyLarge
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Row(
                        modifier = Modifier.align(Alignment.End),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = message.timestampStr,
                            fontSize = 10.sp,
                            color = Color.White.copy(alpha = 0.6f)
                        )
                        if (isOut) {
                            Spacer(modifier = Modifier.width(4.dp))
                            Icon(
                                imageVector = Icons.Default.DoneAll,
                                contentDescription = "Delivered",
                                tint = NodeXCyan,
                                modifier = Modifier.size(13.dp)
                            )
                        }
                    }
                }
            }
        }
    }
}
