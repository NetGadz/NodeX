package network.nodex.messenger.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import network.nodex.messenger.data.NodeContact
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.theme.*

@Composable
fun GroupInfoDialog(
    contact: NodeContact,
    onDismiss: () -> Unit,
    onLeaveGroup: () -> Unit
) {
    val context = LocalContext.current
    val identity by NodeXRepository.identity.collectAsState()
    val isOwner = contact.ownerIdHex.isBlank() || contact.ownerIdHex.equals(identity.nodeId, ignoreCase = true)
    val isChannel = contact.isChannel

    var adminOnlyPosting by remember { mutableStateOf(contact.adminOnlyPosting) }
    var groupTitle by remember { mutableStateOf(contact.name) }
    var groupDesc by remember { mutableStateOf(contact.description.ifBlank { contact.bio }) }
    var isEditing by remember { mutableStateOf(false) }

    Dialog(onDismissRequest = onDismiss) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 24.dp),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(containerColor = NodeXSurface)
        ) {
            Column(
                modifier = Modifier
                    .padding(20.dp)
                    .verticalScroll(rememberScrollState())
            ) {
                // Top Header Row
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = if (isChannel) "Информация о канале" else "Информация о группе",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    IconButton(onClick = onDismiss, modifier = Modifier.size(24.dp)) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Закрыть",
                            tint = TextSecondary
                        )
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Big Avatar & Title
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    Box(
                        modifier = Modifier
                            .size(72.dp)
                            .clip(CircleShape)
                            .background(NodeXPrimary.copy(alpha = 0.15f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = if (isChannel) Icons.Default.Campaign else Icons.Default.Groups,
                            contentDescription = null,
                            tint = NodeXPrimary,
                            modifier = Modifier.size(36.dp)
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    if (isEditing) {
                        OutlinedTextField(
                            value = groupTitle,
                            onValueChange = { groupTitle = it },
                            label = { Text("Название") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedTextColor = TextPrimary,
                                unfocusedTextColor = TextPrimary,
                                focusedBorderColor = NodeXPrimary,
                                unfocusedBorderColor = NodeXBorder
                            )
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        OutlinedTextField(
                            value = groupDesc,
                            onValueChange = { groupDesc = it },
                            label = { Text("Описание") },
                            modifier = Modifier.fillMaxWidth(),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedTextColor = TextPrimary,
                                unfocusedTextColor = TextPrimary,
                                focusedBorderColor = NodeXPrimary,
                                unfocusedBorderColor = NodeXBorder
                            )
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Button(
                            onClick = {
                                NodeXRepository.updateGroupSettings(contact.userIdHex, adminOnlyPosting, groupTitle, groupDesc)
                                isEditing = false
                                Toast.makeText(context, "Настройки сохранены", Toast.LENGTH_SHORT).show()
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = NodeXPrimary),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text("Сохранить изменения")
                        }
                    } else {
                        Text(
                            text = groupTitle,
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Bold,
                            color = TextPrimary
                        )

                        val count = contact.membersCount.coerceAtLeast(1)
                        Text(
                            text = if (isChannel) "$count подписчик" else "$count участник",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextSecondary
                        )

                        if (groupDesc.isNotBlank()) {
                            Spacer(modifier = Modifier.height(6.dp))
                            Text(
                                text = groupDesc,
                                style = MaterialTheme.typography.bodySmall,
                                color = TextMuted
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(20.dp))

                // Invite Link Section
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = CardDefaults.cardColors(containerColor = NodeXBackground)
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Text(
                            text = "Ссылка-приглашение",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextSecondary
                        )
                        Spacer(modifier = Modifier.height(4.dp))
                        val inviteLink = "nodex://join/${contact.userIdHex}"
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = inviteLink,
                                style = MaterialTheme.typography.bodySmall,
                                color = NodeXPrimary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.weight(1f)
                            )
                            IconButton(onClick = {
                                val cm = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                cm.setPrimaryClip(ClipData.newPlainText("Invite Link", inviteLink))
                                Toast.makeText(context, "Ссылка скопирована", Toast.LENGTH_SHORT).show()
                            }) {
                                Icon(
                                    imageVector = Icons.Default.ContentCopy,
                                    contentDescription = "Копировать",
                                    tint = TextSecondary,
                                    modifier = Modifier.size(18.dp)
                                )
                            }
                        }
                    }
                }

                // Owner Settings Section
                if (isOwner) {
                    Spacer(modifier = Modifier.height(16.dp))
                    Text(
                        text = "НАСТРОЙКИ АДМИНИСТРАТОРА",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = "Только администратор пишет",
                                style = MaterialTheme.typography.bodyMedium,
                                color = TextPrimary
                            )
                            Text(
                                text = if (isChannel) "Публикации доступны только вам" else "Запретить участникам отправку сообщений",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextSecondary
                            )
                        }
                        Switch(
                            checked = adminOnlyPosting,
                            onCheckedChange = {
                                adminOnlyPosting = it
                                NodeXRepository.updateGroupSettings(contact.userIdHex, it, groupTitle, groupDesc)
                            },
                            colors = SwitchDefaults.colors(
                                checkedThumbColor = Color.White,
                                checkedTrackColor = NodeXPrimary
                            )
                        )
                    }

                    if (!isEditing) {
                        Spacer(modifier = Modifier.height(8.dp))
                        TextButton(
                            onClick = { isEditing = true },
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Icon(Icons.Default.Edit, contentDescription = null, modifier = Modifier.size(16.dp), tint = NodeXPrimary)
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Редактировать название и описание", color = NodeXPrimary)
                        }
                    }
                }

                Spacer(modifier = Modifier.height(20.dp))

                // Leave / Delete Button
                Button(
                    onClick = {
                        NodeXRepository.deleteContact(contact.userIdHex)
                        onLeaveGroup()
                        onDismiss()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXDestructive.copy(alpha = 0.15f)),
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Icon(
                        imageVector = Icons.Default.DeleteOutline,
                        contentDescription = null,
                        tint = NodeXDestructive,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = if (isOwner) (if (isChannel) "Удалить канал" else "Удалить группу") else (if (isChannel) "Покинуть канал" else "Покинуть группу"),
                        color = NodeXDestructive,
                        fontWeight = FontWeight.SemiBold
                    )
                }
            }
        }
    }
}
