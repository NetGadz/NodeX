package network.nodex.messenger.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import network.nodex.messenger.data.NodeContact
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.theme.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatListScreen(
    onOpenChat: (contactId: String) -> Unit
) {
    val contacts by NodeXRepository.contacts.collectAsState()
    val netStatus by NodeXRepository.networkStatus.collectAsState()
    var showAddDialog by remember { mutableStateOf(false) }

    Scaffold(
        containerColor = NodeXBackground,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = NodeXSurface
                ),
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(38.dp)
                                .clip(RoundedCornerShape(10.dp))
                                .background(
                                    Brush.linearGradient(listOf(NodeXCyan, NodeXBlue))
                                ),
                            contentAlignment = Alignment.Center
                        ) {
                            Icon(
                                imageVector = Icons.Default.Bolt,
                                contentDescription = "NodeX",
                                tint = Color.Black,
                                modifier = Modifier.size(24.dp)
                            )
                        }
                        Spacer(modifier = Modifier.width(12.dp))
                        Column {
                            Text(
                                text = "NodeX",
                                style = MaterialTheme.typography.titleLarge,
                                fontWeight = FontWeight.ExtraBold,
                                color = TextPrimary
                            )
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .size(7.dp)
                                        .clip(CircleShape)
                                        .background(NodeXEmerald)
                                )
                                Spacer(modifier = Modifier.width(5.dp))
                                Text(
                                    text = "${netStatus.bootstrapAddr.split(":")[0]} • ${netStatus.peerCount} peers",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary
                                )
                            }
                        }
                    }
                },
                actions = {
                    IconButton(onClick = {}) {
                        Icon(
                            imageVector = Icons.Default.Shield,
                            contentDescription = "Onion Shield",
                            tint = NodeXCyan
                        )
                    }
                    IconButton(onClick = { showAddDialog = true }) {
                        Icon(
                            imageVector = Icons.Default.QrCodeScanner,
                            contentDescription = "QR / Add",
                            tint = TextPrimary
                        )
                    }
                }
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = { showAddDialog = true },
                containerColor = NodeXCyan,
                contentColor = Color.Black,
                shape = CircleShape
            ) {
                Icon(Icons.Default.Add, contentDescription = "Add Contact")
            }
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(horizontal = 14.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            items(contacts, key = { it.userIdHex }) { contact ->
                ContactItem(contact = contact, onClick = { onOpenChat(contact.userIdHex) })
            }
        }
    }

    if (showAddDialog) {
        AddContactDialog(
            onDismiss = { showAddDialog = false },
            onAddContact = { invite, name ->
                NodeXRepository.addContact(invite, name)
            }
        )
    }
}

@Composable
fun ContactItem(
    contact: NodeContact,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onClick() },
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = NodeXCard),
        border = CardDefaults.outlinedCardBorder().copy(brush = Brush.linearGradient(listOf(NodeXBorder, Color.Transparent)))
    ) {
        Row(
            modifier = Modifier.padding(14.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Avatar
            Box {
                Box(
                    modifier = Modifier
                        .size(50.dp)
                        .clip(RoundedCornerShape(15.dp))
                        .background(NodeXSurfaceLight),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = contact.name.take(1),
                        fontWeight = FontWeight.Bold,
                        fontSize = 20.sp,
                        color = NodeXCyan
                    )
                }
                if (contact.isOnline) {
                    Box(
                        modifier = Modifier
                            .size(13.dp)
                            .align(Alignment.BottomEnd)
                            .clip(CircleShape)
                            .background(NodeXEmerald)
                    )
                }
            }

            Spacer(modifier = Modifier.width(14.dp))

            // Details
            Column(modifier = Modifier.weight(1f)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = contact.name,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    Text(
                        text = contact.lastMessageTime,
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted
                    )
                }
                Spacer(modifier = Modifier.height(4.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = contact.lastMessageText,
                        style = MaterialTheme.typography.bodyMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        color = if (contact.unreadCount > 0) TextPrimary else TextSecondary,
                        modifier = Modifier.weight(1f)
                    )
                    if (contact.unreadCount > 0) {
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(10.dp))
                                .background(Brush.linearGradient(listOf(NodeXCyan, NodeXBlue)))
                                .padding(horizontal = 7.dp, vertical = 2.dp)
                        ) {
                            Text(
                                text = contact.unreadCount.toString(),
                                fontSize = 11.sp,
                                fontWeight = FontWeight.Bold,
                                color = Color.Black
                            )
                        }
                    }
                }
            }
        }
    }
}
