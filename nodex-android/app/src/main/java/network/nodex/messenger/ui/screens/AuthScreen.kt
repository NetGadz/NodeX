package network.nodex.messenger.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.compose.animation.*
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import network.nodex.messenger.crypto.MnemonicHelper
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.components.NodeXBrandMark
import network.nodex.messenger.ui.theme.*

private enum class AuthMode {
    WELCOME,
    CREATE,
    RESTORE
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AuthScreen(
    onAuthenticated: () -> Unit
) {
    val context = LocalContext.current
    var mode by remember { mutableStateOf(AuthMode.WELCOME) }

    // Create account state
    var createName by remember { mutableStateOf("") }
    var generatedMnemonic by remember { mutableStateOf(MnemonicHelper.generateMnemonic(12)) }
    var hasConfirmedSeedBackup by remember { mutableStateOf(false) }

    // Restore account state
    var restoreName by remember { mutableStateOf("") }
    var restoreMnemonicInput by remember { mutableStateOf("") }

    val restoreWords = remember(restoreMnemonicInput) {
        restoreMnemonicInput.trim().split("\\s+".toRegex()).filter { it.isNotBlank() }
    }
    val isRestoreMnemonicValid = remember(restoreWords) {
        restoreWords.size in listOf(12, 24)
    }

    Scaffold(
        containerColor = NodeXBackground,
        topBar = {
            if (mode != AuthMode.WELCOME) {
                TopAppBar(
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = NodeXBackground),
                    navigationIcon = {
                        IconButton(onClick = { mode = AuthMode.WELCOME }) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Назад",
                                tint = TextPrimary
                            )
                        }
                    },
                    title = {
                        Text(
                            text = if (mode == AuthMode.CREATE) "Новый аккаунт" else "Вход в аккаунт",
                            style = MaterialTheme.typography.titleLarge,
                            fontWeight = FontWeight.Bold,
                            color = TextPrimary
                        )
                    }
                )
            }
        }
    ) { paddingValues ->
        AnimatedContent(
            targetState = mode,
            transitionSpec = {
                fadeIn(tween(250)) togetherWith fadeOut(tween(200))
            },
            label = "AuthModeTransition",
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) { currentMode ->
            when (currentMode) {
                AuthMode.WELCOME -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(24.dp)
                            .verticalScroll(rememberScrollState()),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.SpaceBetween
                    ) {
                        Spacer(modifier = Modifier.height(24.dp))

                        // Logo & Brand
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally
                        ) {
                            NodeXBrandMark(size = 116.dp, accent = NodeXBrandCyan, animated = true)

                            Spacer(modifier = Modifier.height(20.dp))

                            Text(
                                text = "NodeX",
                                style = MaterialTheme.typography.headlineMedium,
                                fontWeight = FontWeight.Bold,
                                color = TextPrimary,
                                letterSpacing = 1.5.sp
                            )

                            Spacer(modifier = Modifier.height(6.dp))

                        }

                        // Bottom Action Buttons
                        Column(
                            modifier = Modifier.fillMaxWidth(),
                            verticalArrangement = Arrangement.spacedBy(12.dp)
                        ) {
                            Button(
                                onClick = {
                                    generatedMnemonic = MnemonicHelper.generateMnemonic(12)
                                    hasConfirmedSeedBackup = false
                                    mode = AuthMode.CREATE
                                },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(52.dp),
                                shape = RoundedCornerShape(14.dp),
                                colors = ButtonDefaults.buttonColors(
                                    containerColor = NodeXPrimary,
                                    contentColor = Color.White
                                )
                            ) {
                                Text(
                                    text = "Создать аккаунт",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.SemiBold
                                )
                            }

                            OutlinedButton(
                                onClick = {
                                    restoreMnemonicInput = ""
                                    mode = AuthMode.RESTORE
                                },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .height(52.dp),
                                shape = RoundedCornerShape(14.dp),
                                colors = ButtonDefaults.outlinedButtonColors(
                                    contentColor = TextPrimary
                                ),
                                border = androidx.compose.foundation.BorderStroke(1.dp, NodeXBorder)
                            ) {
                                Text(
                                    text = "Войти",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Medium
                                )
                            }

                            Spacer(modifier = Modifier.height(8.dp))
                        }
                    }
                }

                AuthMode.CREATE -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(horizontal = 20.dp, vertical = 12.dp)
                            .verticalScroll(rememberScrollState()),
                        verticalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        Text(
                            text = "Новый аккаунт",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = TextPrimary
                        )

                        OutlinedTextField(
                            value = createName,
                            onValueChange = { createName = it },
                            label = { Text("Имя") },
                            placeholder = { Text("Введите имя") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(12.dp),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = NodeXPrimary,
                                unfocusedBorderColor = NodeXBorder,
                                focusedLabelColor = NodeXPrimary,
                                unfocusedLabelColor = TextSecondary,
                                focusedTextColor = TextPrimary,
                                unfocusedTextColor = TextPrimary,
                                focusedContainerColor = NodeXSurface,
                                unfocusedContainerColor = NodeXSurface
                            )
                        )

                        // Mnemonic card
                        Card(
                            shape = RoundedCornerShape(14.dp),
                            colors = CardDefaults.cardColors(containerColor = NodeXSurface),
                            border = androidx.compose.foundation.BorderStroke(1.dp, NodeXBorder),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(modifier = Modifier.padding(16.dp)) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(
                                            imageVector = Icons.Default.Key,
                                            contentDescription = null,
                                            tint = NodeXPrimary,
                                            modifier = Modifier.size(18.dp)
                                        )
                                        Spacer(modifier = Modifier.width(8.dp))
                                        Text(
                                            text = "Фраза восстановления",
                                            style = MaterialTheme.typography.titleSmall,
                                            fontWeight = FontWeight.SemiBold,
                                            color = TextPrimary
                                        )
                                    }

                                    IconButton(
                                        onClick = {
                                            generatedMnemonic = MnemonicHelper.generateMnemonic(12)
                                            hasConfirmedSeedBackup = false
                                        },
                                        modifier = Modifier.size(32.dp)
                                    ) {
                                        Icon(
                                            imageVector = Icons.Default.Refresh,
                                            contentDescription = "Сгенерировать другую",
                                            tint = TextSecondary,
                                            modifier = Modifier.size(18.dp)
                                        )
                                    }
                                }

                                Spacer(modifier = Modifier.height(6.dp))

                                Text(
                                    text = "Запишите эти 12 слов по порядку. Они дают полный доступ к вашему аккаунту при смене устройства.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )

                                Spacer(modifier = Modifier.height(14.dp))

                                // Grid of words
                                val words = generatedMnemonic.split(" ").filter { it.isNotBlank() }
                                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                    for (i in 0 until words.size step 2) {
                                        Row(
                                            modifier = Modifier.fillMaxWidth(),
                                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                                        ) {
                                            MnemonicWordChip(
                                                index = i + 1,
                                                word = words[i],
                                                modifier = Modifier.weight(1f)
                                            )
                                            if (i + 1 < words.size) {
                                                MnemonicWordChip(
                                                    index = i + 2,
                                                    word = words[i + 1],
                                                    modifier = Modifier.weight(1f)
                                                )
                                            } else {
                                                Spacer(modifier = Modifier.weight(1f))
                                            }
                                        }
                                    }
                                }

                                Spacer(modifier = Modifier.height(14.dp))

                                OutlinedButton(
                                    onClick = {
                                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                        clipboard.setPrimaryClip(ClipData.newPlainText("NodeX Seed Phrase", generatedMnemonic))
                                        Toast.makeText(context, "12 слов скопированы в буфер", Toast.LENGTH_SHORT).show()
                                    },
                                    modifier = Modifier.fillMaxWidth(),
                                    shape = RoundedCornerShape(10.dp),
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = TextPrimary),
                                    border = androidx.compose.foundation.BorderStroke(1.dp, NodeXBorder)
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.ContentCopy,
                                        contentDescription = null,
                                        modifier = Modifier.size(16.dp)
                                    )
                                    Spacer(modifier = Modifier.width(8.dp))
                                    Text("Скопировать мнемонику", fontSize = 13.sp)
                                }
                            }
                        }

                        // Confirmation Checkbox
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { hasConfirmedSeedBackup = !hasConfirmedSeedBackup }
                                .padding(vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Checkbox(
                                checked = hasConfirmedSeedBackup,
                                onCheckedChange = { hasConfirmedSeedBackup = it },
                                colors = CheckboxDefaults.colors(
                                    checkedColor = NodeXPrimary,
                                    uncheckedColor = NodeXBorder
                                )
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = "Я сохранил(а) эти 12 слов и понимаю, что без них доступ восстановить невозможно.",
                                style = MaterialTheme.typography.bodySmall,
                                color = TextPrimary
                            )
                        }

                        Spacer(modifier = Modifier.height(12.dp))

                        Button(
                            onClick = {
                                val name = if (createName.isNotBlank()) createName.trim() else "Мой аккаунт"
                                NodeXRepository.createAccount(
                                    name = name,
                                    mnemonic = generatedMnemonic
                                )
                                Toast.makeText(context, "Аккаунт успешно создан!", Toast.LENGTH_SHORT).show()
                                onAuthenticated()
                            },
                            enabled = hasConfirmedSeedBackup,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(52.dp),
                            shape = RoundedCornerShape(14.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = NodeXPrimary,
                                contentColor = Color.White,
                                disabledContainerColor = NodeXSurfaceVariant,
                                disabledContentColor = TextMuted
                            )
                        ) {
                            Text(
                                text = "Создать профиль",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.SemiBold
                            )
                        }

                        Spacer(modifier = Modifier.height(16.dp))
                    }
                }

                AuthMode.RESTORE -> {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(horizontal = 20.dp, vertical = 12.dp)
                            .verticalScroll(rememberScrollState()),
                        verticalArrangement = Arrangement.spacedBy(16.dp)
                    ) {
                        Text(
                            text = "Восстановление по мнемонике",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = TextPrimary
                        )

                        Text(
                            text = "Введите 12 слов секретной фразы через пробел для восстановления доступа к аккаунту.",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )

                        OutlinedTextField(
                            value = restoreName,
                            onValueChange = { restoreName = it },
                            label = { Text("Имя профиля (по желанию)") },
                            placeholder = { Text("Мой аккаунт") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(12.dp),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = NodeXPrimary,
                                unfocusedBorderColor = NodeXBorder,
                                focusedLabelColor = NodeXPrimary,
                                unfocusedLabelColor = TextSecondary,
                                focusedTextColor = TextPrimary,
                                unfocusedTextColor = TextPrimary,
                                focusedContainerColor = NodeXSurface,
                                unfocusedContainerColor = NodeXSurface
                            )
                        )

                        OutlinedTextField(
                            value = restoreMnemonicInput,
                            onValueChange = { restoreMnemonicInput = it },
                            label = { Text("12 слов фразы") },
                            placeholder = { Text("word1 word2 word3 ... word12") },
                            minLines = 4,
                            maxLines = 6,
                            modifier = Modifier.fillMaxWidth(),
                            shape = RoundedCornerShape(12.dp),
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedBorderColor = if (isRestoreMnemonicValid) NodeXEmerald else NodeXPrimary,
                                unfocusedBorderColor = if (isRestoreMnemonicValid) NodeXEmerald else NodeXBorder,
                                focusedLabelColor = if (isRestoreMnemonicValid) NodeXEmerald else NodeXPrimary,
                                unfocusedLabelColor = TextSecondary,
                                focusedTextColor = TextPrimary,
                                unfocusedTextColor = TextPrimary,
                                focusedContainerColor = NodeXSurface,
                                unfocusedContainerColor = NodeXSurface
                            )
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = if (isRestoreMnemonicValid) "✓ Распознано 12 слов" else "Слов введено: ${restoreWords.size} / 12",
                                style = MaterialTheme.typography.bodySmall,
                                color = if (isRestoreMnemonicValid) NodeXEmerald else TextMuted,
                                fontWeight = if (isRestoreMnemonicValid) FontWeight.Bold else FontWeight.Normal
                            )

                            TextButton(
                                onClick = {
                                    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                    val clip = clipboard.primaryClip
                                    if (clip != null && clip.itemCount > 0) {
                                        val text = clip.getItemAt(0).text?.toString() ?: ""
                                        if (text.isNotBlank()) {
                                            restoreMnemonicInput = text.trim()
                                        }
                                    }
                                }
                            ) {
                                Icon(
                                    imageVector = Icons.Default.ContentPaste,
                                    contentDescription = null,
                                    modifier = Modifier.size(16.dp),
                                    tint = NodeXPrimary
                                )
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("Вставить из буфера", color = NodeXPrimary, fontSize = 13.sp)
                            }
                        }

                        Spacer(modifier = Modifier.height(6.dp))

                        Button(
                            onClick = {
                                val name = if (restoreName.isNotBlank()) restoreName.trim() else "Мой аккаунт"
                                val success = NodeXRepository.restoreAccount(name, restoreMnemonicInput)
                                if (success) {
                                    Toast.makeText(context, "Вы вошли в аккаунт", Toast.LENGTH_SHORT).show()
                                    onAuthenticated()
                                } else {
                                    Toast.makeText(context, "Ошибка: неверная сид-фраза. Проверьте правильность слов.", Toast.LENGTH_LONG).show()
                                }
                            },
                            enabled = isRestoreMnemonicValid,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(52.dp),
                            shape = RoundedCornerShape(14.dp),
                            colors = ButtonDefaults.buttonColors(
                                containerColor = NodeXPrimary,
                                contentColor = Color.White,
                                disabledContainerColor = NodeXSurfaceVariant,
                                disabledContentColor = TextMuted
                            )
                        ) {
                            Text(
                                text = "Войти по мнемонике",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.SemiBold
                            )
                        }

                        Spacer(modifier = Modifier.height(16.dp))
                    }
                }
            }
        }
    }
}

@Composable
private fun MnemonicWordChip(
    index: Int,
    word: String,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(8.dp))
            .background(NodeXSurfaceVariant)
            .border(1.dp, NodeXBorder, RoundedCornerShape(8.dp))
            .padding(horizontal = 10.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = "$index.",
            style = MaterialTheme.typography.labelSmall,
            color = TextMuted,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.width(22.dp)
        )
        Text(
            text = word,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium,
            color = TextPrimary,
            fontFamily = FontFamily.Monospace
        )
    }
}
