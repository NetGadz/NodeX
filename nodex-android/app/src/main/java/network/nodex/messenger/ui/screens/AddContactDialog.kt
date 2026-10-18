package network.nodex.messenger.ui.screens

import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentPaste
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.theme.*

@Composable
fun AddContactDialog(
    onDismiss: () -> Unit,
    onContactAdded: (String) -> Unit = {}
) {
    val context = LocalContext.current
    var inputIdOrInvite by remember { mutableStateOf("") }
    var inputName by remember { mutableStateOf("") }

    Dialog(onDismissRequest = onDismiss) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            shape = RoundedCornerShape(20.dp),
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
                        text = "Новый контакт",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary,
                        fontSize = 18.sp
                    )
                    IconButton(onClick = onDismiss, modifier = Modifier.size(26.dp)) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Закрыть",
                            tint = TextSecondary
                        )
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                OutlinedTextField(
                    value = inputName,
                    onValueChange = { inputName = it },
                    label = { Text("Имя контакта (необязательно)") },
                    placeholder = { Text("Например: Друг") },
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
                    shape = RoundedCornerShape(12.dp)
                )

                Spacer(modifier = Modifier.height(12.dp))

                OutlinedTextField(
                    value = inputIdOrInvite,
                    onValueChange = { inputIdOrInvite = it },
                    label = { Text("ID контакта или ссылка-приглашение") },
                    placeholder = { Text("nodex://invite/... или hex") },
                    singleLine = true,
                    trailingIcon = {
                        IconButton(onClick = {
                            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                            val clipText = clipboard?.primaryClip?.getItemAt(0)?.text?.toString()?.trim()
                            if (!clipText.isNullOrBlank()) {
                                inputIdOrInvite = clipText
                                Toast.makeText(context, "Вставлено из буфера", Toast.LENGTH_SHORT).show()
                            }
                        }) {
                            Icon(
                                imageVector = Icons.Default.ContentPaste,
                                contentDescription = "Вставить",
                                tint = NodeXPrimary
                            )
                        }
                    },
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = NodeXPrimary,
                        unfocusedBorderColor = NodeXBorder,
                        focusedLabelColor = NodeXPrimary,
                        unfocusedLabelColor = TextSecondary
                    ),
                    shape = RoundedCornerShape(12.dp)
                )

                Spacer(modifier = Modifier.height(20.dp))

                Button(
                    onClick = {
                        val clean = inputIdOrInvite.trim()
                        if (clean.isBlank()) {
                            Toast.makeText(context, "Введите ID контакта или ссылку", Toast.LENGTH_SHORT).show()
                            return@Button
                        }
                        val parsedId = if (clean.startsWith("nodex://invite/")) {
                            network.nodex.messenger.crypto.AndroidMessageCrypto.inviteNodeId(clean) ?: clean.removePrefix("nodex://invite/").trim()
                        } else {
                            clean
                        }
                        NodeXRepository.addContact(clean, inputName.trim())
                        Toast.makeText(context, "Контакт добавлен", Toast.LENGTH_SHORT).show()
                        onContactAdded(parsedId)
                        onDismiss()
                    },
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXPrimary)
                ) {
                    Text(text = "Добавить в чаты", fontWeight = FontWeight.SemiBold, fontSize = 15.sp)
                }
            }
        }
    }
}
