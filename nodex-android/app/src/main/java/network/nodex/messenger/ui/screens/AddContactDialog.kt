package network.nodex.messenger.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import network.nodex.messenger.ui.theme.*

@Composable
fun AddContactDialog(
    onDismiss: () -> Unit,
    onAddContact: (invite: String, name: String) -> Unit
) {
    var inviteInput by remember { mutableStateOf("") }
    var nameInput by remember { mutableStateOf("") }

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = NodeXCard,
            tonalElevation = 8.dp,
            modifier = Modifier.fillMaxWidth().padding(8.dp)
        ) {
            Column(
                modifier = Modifier.padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp)
            ) {
                Text(
                    text = "Добавить P2P контакт",
                    style = MaterialTheme.typography.titleLarge,
                    color = TextPrimary
                )

                Text(
                    text = "Введите Invite-ссылку (nodex://invite/...) или ID узла в сети Kademlia:",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary
                )

                OutlinedTextField(
                    value = nameInput,
                    onValueChange = { nameInput = it },
                    label = { Text("Имя контакта (псевдоним)") },
                    placeholder = { Text("Например: Алиса") },
                    singleLine = true,
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = NodeXCyan,
                        unfocusedBorderColor = NodeXBorder,
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary
                    ),
                    modifier = Modifier.fillMaxWidth()
                )

                OutlinedTextField(
                    value = inviteInput,
                    onValueChange = { inviteInput = it },
                    label = { Text("Invite-ссылка / Node ID") },
                    placeholder = { Text("nodex://invite/... или 40 hex") },
                    singleLine = true,
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedBorderColor = NodeXCyan,
                        unfocusedBorderColor = NodeXBorder,
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary
                    ),
                    modifier = Modifier.fillMaxWidth()
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End
                ) {
                    TextButton(onClick = onDismiss) {
                        Text("Отмена", color = TextMuted)
                    }
                    Spacer(modifier = Modifier.width(8.dp))
                    Button(
                        onClick = {
                            if (inviteInput.isNotBlank() && nameInput.isNotBlank()) {
                                onAddContact(inviteInput.trim(), nameInput.trim())
                                onDismiss()
                            }
                        },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = NodeXCyan,
                            contentColor = Color.Black
                        ),
                        enabled = inviteInput.isNotBlank() && nameInput.isNotBlank()
                    ) {
                        Text("Добавить")
                    }
                }
            }
        }
    }
}
