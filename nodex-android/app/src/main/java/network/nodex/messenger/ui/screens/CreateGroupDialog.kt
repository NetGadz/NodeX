package network.nodex.messenger.ui.screens

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Campaign
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Groups
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.theme.*

@Composable
fun CreateGroupDialog(
    onDismiss: () -> Unit,
    onGroupCreated: (groupId: String) -> Unit
) {
    val context = LocalContext.current
    val contacts by NodeXRepository.contacts.collectAsState()
    val otherContacts = remember(contacts) { contacts.filter { !it.isSelf && !it.isChannel && !it.isGroup } }

    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var isChannel by remember { mutableStateOf(false) } // false = Group, true = Channel
    val selectedMembers = remember { mutableStateListOf<String>() }

    Dialog(onDismissRequest = onDismiss) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.cardColors(containerColor = NodeXSurface),
            border = null
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = if (isChannel) "Новый канал" else "Новая группа",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    IconButton(onClick = onDismiss, modifier = Modifier.size(24.dp)) {
                        Icon(imageVector = Icons.Default.Close, contentDescription = "Закрыть", tint = TextSecondary)
                    }
                }

                Spacer(modifier = Modifier.height(14.dp))

                // Group vs Channel Switcher Tab
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(10.dp))
                        .background(NodeXBackground)
                        .padding(3.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(8.dp))
                            .background(if (!isChannel) NodeXPrimary else Color.Transparent)
                            .clickable { isChannel = false }
                            .padding(vertical = 8.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Groups,
                                contentDescription = null,
                                tint = if (!isChannel) Color.White else TextMuted,
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = "Группа",
                                color = if (!isChannel) Color.White else TextMuted,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 13.sp
                            )
                        }
                    }

                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(8.dp))
                            .background(if (isChannel) NodeXPrimary else Color.Transparent)
                            .clickable { isChannel = true }
                            .padding(vertical = 8.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Icon(
                                imageVector = Icons.Default.Campaign,
                                contentDescription = null,
                                tint = if (isChannel) Color.White else TextMuted,
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = "Канал",
                                color = if (isChannel) Color.White else TextMuted,
                                fontWeight = FontWeight.SemiBold,
                                fontSize = 13.sp
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(14.dp))

                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    label = { Text(if (isChannel) "Название канала" else "Название группы") },
                    placeholder = { Text(if (isChannel) "Новости и статьи" else "Командный чат") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = NodeXPrimary,
                        unfocusedBorderColor = NodeXBorder,
                        focusedLabelColor = NodeXPrimary,
                        unfocusedLabelColor = TextSecondary
                    ),
                    shape = RoundedCornerShape(10.dp)
                )

                Spacer(modifier = Modifier.height(10.dp))

                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("Описание (необязательно)") },
                    placeholder = { Text(if (isChannel) "О чем этот канал" else "Тема общения") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = NodeXPrimary,
                        unfocusedBorderColor = NodeXBorder,
                        focusedLabelColor = NodeXPrimary,
                        unfocusedLabelColor = TextSecondary
                    ),
                    shape = RoundedCornerShape(10.dp)
                )

                Spacer(modifier = Modifier.height(12.dp))

                Text(
                    text = if (isChannel) "Подписчики:" else "Участники:",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted
                )

                Spacer(modifier = Modifier.height(6.dp))

                if (otherContacts.isEmpty()) {
                    Text(
                        text = "Вы являетесь создателем и администратором.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary,
                        modifier = Modifier.padding(vertical = 4.dp)
                    )
                } else {
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(max = 120.dp)
                    ) {
                        items(otherContacts, key = { it.userIdHex }) { contact ->
                            val isSelected = selectedMembers.contains(contact.userIdHex)
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        if (isSelected) {
                                            selectedMembers.remove(contact.userIdHex)
                                        } else {
                                            selectedMembers.add(contact.userIdHex)
                                        }
                                    }
                                    .padding(vertical = 6.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(22.dp)
                                        .clip(CircleShape)
                                        .background(if (isSelected) NodeXPrimary else NodeXBorder),
                                    contentAlignment = Alignment.Center
                                ) {
                                    if (isSelected) {
                                        Icon(
                                            imageVector = Icons.Default.Check,
                                            contentDescription = null,
                                            tint = Color.White,
                                            modifier = Modifier.size(14.dp)
                                        )
                                    }
                                }
                                Spacer(modifier = Modifier.width(10.dp))
                                Text(
                                    text = contact.name,
                                    color = TextPrimary,
                                    style = MaterialTheme.typography.bodyMedium
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(18.dp))

                Button(
                    onClick = {
                        val cleanTitle = title.trim()
                        if (cleanTitle.isEmpty()) {
                            Toast.makeText(context, "Введите название", Toast.LENGTH_SHORT).show()
                            return@Button
                        }
                        val cleanDesc = description.trim()
                        val createdId = if (isChannel) {
                            NodeXRepository.createChannel(title = cleanTitle, description = cleanDesc)
                        } else {
                            NodeXRepository.createGroup(title = cleanTitle, description = cleanDesc)
                        }
                        Toast.makeText(context, if (isChannel) "Канал создан" else "Группа создана", Toast.LENGTH_SHORT).show()
                        onGroupCreated(createdId)
                    },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(10.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXPrimary)
                ) {
                    Text(
                        text = if (isChannel) "Создать канал" else "Создать группу",
                        fontWeight = FontWeight.SemiBold
                    )
                }
            }
        }
    }
}
