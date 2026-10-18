package network.nodex.messenger.ui.screens

import android.Manifest
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.pm.PackageManager
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.*
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.core.content.ContextCompat
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.provider.OpenableColumns
import androidx.compose.foundation.Image
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import network.nodex.messenger.data.ChatMessage
import network.nodex.messenger.data.ChatTheme
import network.nodex.messenger.data.ChatType
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.data.P2PTransportEngine
import network.nodex.messenger.ui.components.VoiceMessagePlayer
import network.nodex.messenger.ui.components.VoiceRecordBar
import network.nodex.messenger.ui.theme.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatDetailScreen(
    contactId: String,
    onBack: () -> Unit,
    onStartCall: (peerId: String, peerName: String) -> Unit = { _, _ -> }
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val keyboardController = LocalSoftwareKeyboardController.current
    val currentTheme by NodeXRepository.currentTheme.collectAsState()
    val customAccentHex by NodeXRepository.customAccentHex.collectAsState()
    val customAccent = customAccentHex?.let {
        try { Color(android.graphics.Color.parseColor(it)) } catch (e: Exception) { null }
    }
    val palette = getThemeColors(currentTheme, customAccent)
    val chatFontSize by NodeXRepository.chatFontSize.collectAsState()
    val bubbleRounding by NodeXRepository.bubbleRounding.collectAsState()

    val contacts by NodeXRepository.contacts.collectAsState()
    val allMessages by NodeXRepository.messages.collectAsState()
    val contact = contacts.firstOrNull { it.userIdHex.equals(contactId, ignoreCase = true) }
    val messages = allMessages[contactId] ?: emptyList()

    val identity by NodeXRepository.identity.collectAsState()
    val isSelfChat = contact?.isSelf == true || contactId.equals(identity.nodeId, ignoreCase = true) || contactId == "self_saved_messages"
    val isChannel = contact?.isChannel == true || contact?.chatType == ChatType.CHANNEL
    val isGroup = contact?.isGroup == true || contact?.chatType == ChatType.GROUP
    val isGroupOrChannel = isChannel || isGroup

    // Call icon is strictly for 1-to-1 direct peer chats! Never for channels or groups!
    val canCall = !isSelfChat && !isGroupOrChannel

    // Keep the header presence and encryption keys fresh while a chat is open.
    // Persisted contact state can otherwise show “recently” until the next
    // background poll, especially after returning from Android Studio.
    LaunchedEffect(contactId, isSelfChat, isGroupOrChannel) {
        if (!isSelfChat && !isGroupOrChannel) {
            while (isActive) {
                P2PTransportEngine.queryPeerPresence(contactId)
                delay(5_000)
            }
        }
    }

    // Posting restrictions for channels and muted groups
    val canPost = NodeXRepository.canUserPost(contactId)

    LaunchedEffect(contactId) {
        NodeXRepository.markChatAsRead(contactId)
    }

    var inputText by remember { mutableStateOf("") }
    var isRecordingVoice by remember { mutableStateOf(false) }
    var showContactInfoDialog by remember { mutableStateOf(false) }
    var showGroupInfoDialog by remember { mutableStateOf(false) }
    var showAttachmentMenu by remember { mutableStateOf(false) }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        if (isGranted) {
            isRecordingVoice = true
        } else {
            Toast.makeText(context, "Для записи голосовых сообщений требуется доступ к микрофону", Toast.LENGTH_SHORT).show()
        }
    }

    // Photo/Image picker from Gallery
    val photoPickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        if (uri != null) {
            scope.launch(Dispatchers.IO) {
                try {
                    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    context.contentResolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, bounds) }
                    var sample = 1
                    while (bounds.outWidth / sample > 2048 || bounds.outHeight / sample > 2048) sample *= 2
                    val options = BitmapFactory.Options().apply { inSampleSize = sample.coerceAtLeast(1) }
                    val rawBitmap = context.contentResolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, options) }

                    if (rawBitmap != null) {
                        val maxDim = 1280
                        val width = rawBitmap.width
                        val height = rawBitmap.height
                        val ratio = if (width > height) {
                            maxDim.toFloat() / width.toFloat()
                        } else {
                            maxDim.toFloat() / height.toFloat()
                        }
                        val scaledBitmap = if (ratio < 1.0f) {
                            Bitmap.createScaledBitmap(rawBitmap, (width * ratio).toInt(), (height * ratio).toInt(), true)
                        } else {
                            rawBitmap
                        }

                        val imgDir = File(context.filesDir, "media_images").apply { mkdirs() }
                        val localFile = File(imgDir, "send_${System.currentTimeMillis()}.jpg")
                        val out = java.io.FileOutputStream(localFile)
                        scaledBitmap.compress(Bitmap.CompressFormat.JPEG, 75, out)
                        out.flush()
                        out.close()

                        withContext(Dispatchers.Main) {
                            NodeXRepository.sendImageMessage(contactId, localFile.absolutePath)
                        }
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
        }
    }

    // Generic document/file picker
    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent()
    ) { uri: Uri? ->
        if (uri != null) {
            scope.launch(Dispatchers.IO) {
                try {
                    var fileName = "file_${System.currentTimeMillis()}"
                    var fileSize = 0L

                    context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
                        val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                        val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
                        if (cursor.moveToFirst()) {
                            if (nameIndex >= 0) fileName = cursor.getString(nameIndex) ?: fileName
                            if (sizeIndex >= 0) fileSize = cursor.getLong(sizeIndex)
                        }
                    }

                    if (fileSize > 20L * 1024 * 1024) {
                        withContext(Dispatchers.Main) { Toast.makeText(context, "Максимальный размер файла — 20 МБ", Toast.LENGTH_LONG).show() }
                        return@launch
                    }

                    val inputStream = context.contentResolver.openInputStream(uri)
                    val fileDir = File(context.filesDir, "media_files").apply { mkdirs() }
                    val safeName = fileName.replace(Regex("[^a-zA-Z0-9._-]"), "_")
                    val localFile = File(fileDir, "send_${System.currentTimeMillis()}_$safeName")
                    var oversized = false
                    inputStream?.use { source ->
                        java.io.FileOutputStream(localFile).use { target ->
                            val buffer = ByteArray(16 * 1024)
                            var total = 0L
                            while (true) {
                                val count = source.read(buffer)
                                if (count < 0) break
                                total += count
                                if (total > 20L * 1024 * 1024) { oversized = true; break }
                                target.write(buffer, 0, count)
                            }
                        }
                    }

                    if (oversized || localFile.length() > 20L * 1024 * 1024) {
                        localFile.delete()
                        withContext(Dispatchers.Main) { Toast.makeText(context, "Максимальный размер файла — 20 МБ", Toast.LENGTH_LONG).show() }
                        return@launch
                    }

                    val sizeStr = when {
                        fileSize > 1024 * 1024 -> "%.1f МБ".format(fileSize.toFloat() / (1024 * 1024))
                        fileSize > 1024 -> "%d КБ".format(fileSize / 1024)
                        else -> "%d Б".format(fileSize)
                    }

                    withContext(Dispatchers.Main) {
                        NodeXRepository.sendFileMessage(contactId, localFile.absolutePath, fileName, sizeStr)
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
        }
    }

    val doSendMessage = {
        val textToSend = inputText.trim()
        if (textToSend.isNotEmpty()) {
            NodeXRepository.sendMessage(contactId, textToSend)
            inputText = ""
        }
        keyboardController?.hide()
    }

    Scaffold(
        containerColor = palette.background,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.surface),
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Назад",
                            tint = TextPrimary
                        )
                    }
                },
                title = {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                if (isGroupOrChannel) {
                                    showGroupInfoDialog = true
                                } else if (!isSelfChat) {
                                    showContactInfoDialog = true
                                }
                            },
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // Avatar Icon or Letter
                        val avatarBrush = when {
                            isSelfChat -> Brush.linearGradient(listOf(palette.primary, palette.primary.copy(alpha = 0.38f)))
                            isChannel -> Brush.linearGradient(listOf(Color(0xFFC2A0FF), Color(0xFF4A3670)))
                            isGroup -> Brush.linearGradient(listOf(Color(0xFF80E0B3), Color(0xFF205044)))
                            else -> Brush.linearGradient(listOf(palette.primary, palette.primary.copy(alpha = 0.28f)))
                        }

                        Box(
                            modifier = Modifier
                                .size(44.dp)
                                .clip(RoundedCornerShape(16.dp))
                                .background(avatarBrush),
                            contentAlignment = Alignment.Center
                        ) {
                            if (isSelfChat) {
                                Icon(
                                    imageVector = Icons.Default.Bookmark,
                                    contentDescription = "Избранное",
                                    tint = Color.White,
                                    modifier = Modifier.size(20.dp)
                                )
                            } else if (isChannel) {
                                Icon(
                                    imageVector = Icons.Default.Campaign,
                                    contentDescription = "Канал",
                                    tint = Color.White,
                                    modifier = Modifier.size(22.dp)
                                )
                            } else if (isGroup) {
                                Icon(
                                    imageVector = Icons.Default.Groups,
                                    contentDescription = "Группа",
                                    tint = Color.White,
                                    modifier = Modifier.size(22.dp)
                                )
                            } else {
                                val cleanLetter = (contact?.name ?: "U")
                                    .filter { it.isLetterOrDigit() }
                                    .take(1)
                                    .ifEmpty { "U" }
                                    .uppercase()
                                Text(
                                    text = cleanLetter,
                                    color = Color.White,
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 18.sp
                                )
                            }
                        }

                        Spacer(modifier = Modifier.width(12.dp))

                        Column {
                            val chatTitle = if (isSelfChat) {
                                "${identity.name} (Вы)"
                            } else {
                                contact?.name ?: "Контакт ${contactId.take(8)}"
                            }

                            Text(
                                text = chatTitle,
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = TextPrimary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )

                            // Subtitle: followers for channels, members for groups, online status for contacts
                            if (isSelfChat) {
                                Text(
                                    text = "избранное • личные заметки",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary,
                                    fontSize = 11.sp
                                )
                            } else if (isChannel) {
                                val count = contact?.membersCount ?: 1
                                Text(
                                    text = "$count подписчиков",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary,
                                    fontSize = 11.sp
                                )
                            } else if (isGroup) {
                                val count = contact?.membersCount ?: 1
                                Text(
                                    text = "$count участников",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary,
                                    fontSize = 11.sp
                                )
                            } else if (contact?.isIdentityVerified == true) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Icon(Icons.Default.VerifiedUser, contentDescription = null, tint = palette.primary, modifier = Modifier.size(12.dp))
                                    Spacer(modifier = Modifier.width(4.dp))
                                    Text(if (contact.isOnline) "в сети" else "был(а) недавно", style = MaterialTheme.typography.labelSmall, color = palette.primary, fontSize = 10.sp)
                                }
                            } else if (contact?.isOnline == true) {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    Box(
                                        modifier = Modifier
                                            .size(7.dp)
                                            .clip(CircleShape)
                                            .background(NodeXEmerald)
                                    )
                                    Spacer(modifier = Modifier.width(5.dp))
                                    Text(
                                        text = "в сети",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = NodeXEmerald,
                                        fontSize = 11.sp
                                    )
                                }
                            } else {
                                Text(
                                    text = "был(а) недавно",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary,
                                    fontSize = 11.sp
                                )
                            }
                        }
                    }
                },
                actions = {
                    // Call button is ONLY shown for direct 1-to-1 peer chats!
                    if (canCall) {
                        IconButton(onClick = {
                            val name = contact?.name ?: "Контакт ${contactId.take(8)}"
                            NodeXRepository.startCall(contactId, name)
                            onStartCall(contactId, name)
                        }) {
                            Icon(
                                imageVector = Icons.Default.Call,
                                contentDescription = "Позвонить",
                                tint = palette.primary,
                                modifier = Modifier.size(24.dp)
                            )
                        }
                    }

                    // More / Profile details button
                    IconButton(onClick = {
                        if (isGroupOrChannel) {
                            showGroupInfoDialog = true
                        } else if (!isSelfChat) {
                            showContactInfoDialog = true
                        }
                    }) {
                        Icon(
                            imageVector = Icons.Default.MoreVert,
                            contentDescription = "Информация",
                            tint = TextSecondary,
                            modifier = Modifier.size(22.dp)
                        )
                    }
                }
            )
        },
        bottomBar = {
            Surface(
                color = palette.background,
                modifier = Modifier
                    .fillMaxWidth()
                    .imePadding()
                    .navigationBarsPadding()
            ) {
                if (!canPost) {
                    Surface(
                        color = palette.surface,
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 14.dp, vertical = 8.dp),
                        shape = RoundedCornerShape(16.dp)
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.Center
                        ) {
                            Icon(
                                imageVector = Icons.Default.Campaign,
                                contentDescription = null,
                                tint = TextMuted,
                                modifier = Modifier.size(20.dp)
                            )
                            Spacer(modifier = Modifier.width(10.dp))
                            Text(
                                text = if (isChannel) "📢 Публикация доступна только администраторам" else "💬 Отправка сообщений ограничена",
                                style = MaterialTheme.typography.bodyMedium,
                                color = TextMuted
                            )
                        }
                    }
                } else if (isRecordingVoice) {
                    Box(modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp)) {
                        VoiceRecordBar(
                            isRecording = true,
                            onCancelRecording = { isRecordingVoice = false },
                            onSendVoice = { duration, wave, audioPath ->
                                NodeXRepository.sendVoiceMessage(contactId, duration, wave, audioPath)
                                isRecordingVoice = false
                            }
                        )
                    }
                } else {
                    Column(modifier = Modifier.fillMaxWidth()) {
                        // Live Message Preview Box ("Окно просмотра текста")
                        AnimatedVisibility(
                            visible = inputText.isNotBlank(),
                            enter = fadeIn() + expandVertically(),
                            exit = fadeOut() + shrinkVertically()
                        ) {
                            Surface(
                                color = palette.surface,
                                shape = RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(horizontal = 12.dp)
                                    .border(
                                        width = 1.dp,
                                        color = palette.primary.copy(alpha = 0.45f),
                                        shape = RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)
                                    )
                            ) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(horizontal = 14.dp, vertical = 8.dp)
                                        .heightIn(max = 120.dp)
                                        .verticalScroll(rememberScrollState())
                                ) {
                                    Text(
                                        text = inputText,
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = TextPrimary,
                                        fontSize = 15.sp,
                                        lineHeight = 20.sp
                                    )
                                }
                            }
                        }

                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 12.dp, vertical = 8.dp)
                                .clip(RoundedCornerShape(26.dp))
                                .background(palette.surface.copy(alpha = 0.88f))
                                .padding(horizontal = 4.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            // Paperclip Attachment Button
                            IconButton(
                                onClick = { showAttachmentMenu = true },
                                modifier = Modifier.size(42.dp)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.AttachFile,
                                    contentDescription = "Прикрепить",
                                    tint = TextSecondary
                                )
                            }

                            // Input Field
                            TextField(
                                value = inputText,
                                onValueChange = { newText ->
                                    if (newText.contains("\n")) {
                                        val cleaned = newText.replace("\n", "").trim()
                                        if (cleaned.isNotBlank()) {
                                            inputText = cleaned
                                            doSendMessage()
                                        } else {
                                            inputText = ""
                                            keyboardController?.hide()
                                        }
                                    } else {
                                        inputText = newText
                                    }
                                },
                                placeholder = { Text("Сообщение...", color = TextMuted, fontSize = 15.sp) },
                                modifier = Modifier
                                    .weight(1f)
                                    .clip(RoundedCornerShape(24.dp)),
                                keyboardOptions = KeyboardOptions(
                                    imeAction = ImeAction.Send,
                                    capitalization = KeyboardCapitalization.Sentences
                                ),
                                keyboardActions = KeyboardActions(
                                    onSend = { doSendMessage() }
                                ),
                                colors = TextFieldDefaults.colors(
                                    focusedContainerColor = Color.Transparent,
                                    unfocusedContainerColor = Color.Transparent,
                                    focusedTextColor = TextPrimary,
                                    unfocusedTextColor = TextPrimary,
                                    focusedIndicatorColor = Color.Transparent,
                                    unfocusedIndicatorColor = Color.Transparent
                                ),
                                singleLine = false,
                                maxLines = 4
                            )

                            Spacer(modifier = Modifier.width(8.dp))

                            // Animated Mic / Send Button (Telegram style morph)
                            AnimatedContent(
                                targetState = inputText.isNotBlank(),
                                transitionSpec = {
                                    (scaleIn() + fadeIn()).togetherWith(scaleOut() + fadeOut())
                                },
                                label = "SendMicTransition"
                            ) { hasText ->
                                if (hasText) {
                                    Box(
                                        modifier = Modifier
                                            .size(44.dp)
                                            .clip(CircleShape)
                                            .background(palette.primary)
                                            .clickable { doSendMessage() },
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Icon(
                                            imageVector = Icons.AutoMirrored.Filled.Send,
                                            contentDescription = "Отправить",
                                            tint = Color.White,
                                            modifier = Modifier.size(20.dp)
                                        )
                                    }
                                } else {
                                    Box(
                                        modifier = Modifier
                                            .size(44.dp)
                                            .clip(CircleShape)
                                            .background(palette.surface)
                                            .clickable {
                                                val hasPermission = ContextCompat.checkSelfPermission(
                                                    context,
                                                    Manifest.permission.RECORD_AUDIO
                                                ) == PackageManager.PERMISSION_GRANTED
                                                if (hasPermission) {
                                                    isRecordingVoice = true
                                                } else {
                                                    permissionLauncher.launch(Manifest.permission.RECORD_AUDIO)
                                                }
                                            },
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Icon(
                                            imageVector = Icons.Default.Mic,
                                            contentDescription = "Голосовое сообщение",
                                            tint = palette.primary,
                                            modifier = Modifier.size(22.dp)
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            if (messages.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            imageVector = Icons.Default.Lock,
                            contentDescription = null,
                            tint = NodeXPrimary.copy(alpha = 0.5f),
                            modifier = Modifier.size(36.dp)
                        )
                        Spacer(modifier = Modifier.height(10.dp))
                        Text(
                            text = if (isChannel) "В канале пока нет публикаций" else "Начните разговор или отправьте голосовое сообщение",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextMuted,
                            textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                            fontSize = 14.sp
                        )
                    }
                }
            } else {
                val listState = rememberLazyListState()
                LaunchedEffect(messages.size) {
                    if (messages.isNotEmpty()) {
                        listState.animateScrollToItem(messages.size - 1)
                    }
                }
                LazyColumn(
                    state = listState,
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    reverseLayout = false
                ) {
                    items(messages, key = { it.id }) { msg ->
                        AnimatedVisibility(visible = true, enter = fadeIn() + slideInVertically(initialOffsetY = { it / 8 })) {
                            DetailedMessageBubble(
                                message = msg,
                                showSenderName = isGroup && !msg.isOutgoing,
                                palette = palette,
                                fontSizeSp = chatFontSize,
                                roundingDp = bubbleRounding
                            )
                        }
                    }
                }
            }
        }
    }

    // Attachment Menu Dialog
    if (showAttachmentMenu) {
        AlertDialog(
            onDismissRequest = { showAttachmentMenu = false },
            title = { Text("Прикрепить файл") },
            text = {
                Column {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp))
                            .clickable {
                                showAttachmentMenu = false
                                photoPickerLauncher.launch("image/*")
                            }
                            .padding(vertical = 12.dp, horizontal = 4.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(imageVector = Icons.Default.Image, contentDescription = null, tint = palette.primary)
                        Spacer(modifier = Modifier.width(14.dp))
                        Text("Фото или видео из галереи", color = TextPrimary)
                    }

                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp))
                            .clickable {
                                showAttachmentMenu = false
                                filePickerLauncher.launch("*/*")
                            }
                            .padding(vertical = 12.dp, horizontal = 4.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(imageVector = Icons.Default.InsertDriveFile, contentDescription = null, tint = NodeXEmerald)
                        Spacer(modifier = Modifier.width(14.dp))
                        Text("Файл / Документ", color = TextPrimary)
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = { showAttachmentMenu = false }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // Group / Channel Info Dialog
    if (showGroupInfoDialog && contact != null) {
        GroupInfoDialog(
            contact = contact,
            onDismiss = { showGroupInfoDialog = false },
            onLeaveGroup = {
                showGroupInfoDialog = false
                onBack()
            }
        )
    }

    // Contact Info Dialog (1-to-1)
    if (showContactInfoDialog) {
        ContactInfoDialog(
            contactName = contact?.name ?: "Контакт",
            contactIdHex = contactId,
            onClearHistory = {
                NodeXRepository.clearMessages(contactId)
                showContactInfoDialog = false
            },
            onDeleteContact = {
                NodeXRepository.deleteContact(contactId)
                showContactInfoDialog = false
                onBack()
            },
            onDismiss = { showContactInfoDialog = false }
        )
    }
}

@Composable
fun DetailedMessageBubble(
    message: ChatMessage,
    showSenderName: Boolean = false,
    palette: ThemePalette = getThemeColors(ChatTheme.GRAPHITE),
    fontSizeSp: Float = 15f,
    roundingDp: Float = 18f
) {
    val context = LocalContext.current
    val isMe = message.isOutgoing
    val isCallMessage = message.text.contains("вызов") || message.text.contains("звонок")

    if (isCallMessage) {
        // Center-aligned call event badge
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 4.dp),
            horizontalArrangement = Arrangement.Center
        ) {
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(14.dp))
                    .background(palette.surface.copy(alpha = 0.85f))
                    .padding(horizontal = 14.dp, vertical = 6.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = message.text,
                        style = MaterialTheme.typography.bodySmall,
                        color = Color.White.copy(alpha = 0.9f),
                        fontSize = 13.sp
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = message.timestampStr,
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted,
                        fontSize = 10.sp
                    )
                }
            }
        }
        return
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isMe) Arrangement.End else Arrangement.Start
    ) {
        val cornerTopStart = roundingDp.dp
        val cornerTopEnd = roundingDp.dp
        val cornerBottomStart = if (isMe) roundingDp.dp else (roundingDp * 0.25f).dp
        val cornerBottomEnd = if (isMe) (roundingDp * 0.25f).dp else roundingDp.dp

        Box(
            modifier = Modifier
                .widthIn(min = 90.dp, max = 310.dp)
                .animateContentSize()
                .clip(
                    RoundedCornerShape(
                        topStart = cornerTopStart,
                        topEnd = cornerTopEnd,
                        bottomStart = cornerBottomStart,
                        bottomEnd = cornerBottomEnd
                    )
                )
                .background(if (isMe) palette.bubbleOut else palette.bubbleIn)
                .padding(horizontal = 12.dp, vertical = 8.dp)
        ) {
            Column {
                if (showSenderName && !message.senderName.isNullOrBlank()) {
                    Text(
                        text = message.senderName,
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = palette.primary,
                        modifier = Modifier.padding(bottom = 2.dp)
                    )
                }

                if (message.isVoice) {
                    VoiceMessagePlayer(
                        messageId = message.id,
                        durationSecs = message.voiceDurationSecs,
                        waveform = message.waveform,
                        isMe = isMe,
                        audioFilePath = message.audioFilePath
                    )
                } else if (message.isImage && message.imageFilePath.isNotBlank()) {
                    val bitmap = remember(message.imageFilePath) {
                        try {
                            val f = File(message.imageFilePath)
                            if (f.exists() && f.length() > 50L) {
                                BitmapFactory.decodeFile(f.absolutePath)?.asImageBitmap()
                            } else null
                        } catch (e: Exception) {
                            null
                        }
                    }
                    if (bitmap != null) {
                        Image(
                            bitmap = bitmap,
                            contentDescription = "Фотография",
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(max = 240.dp)
                                .clip(RoundedCornerShape(12.dp))
                                .clickable {
                                    openFileWithDefaultApp(context, File(message.imageFilePath), "image/jpeg")
                                },
                            contentScale = ContentScale.Crop
                        )
                    } else {
                        Text(
                            text = "📷 Фотография",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextPrimary
                        )
                    }
                    if (message.text.isNotBlank() && message.text != "📷 Фотография") {
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            text = message.text,
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextPrimary,
                            fontSize = 15.sp,
                            lineHeight = 20.sp
                        )
                    }
                } else if (message.isFile && message.filePath.isNotBlank()) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(12.dp))
                            .background(palette.background.copy(alpha = 0.5f))
                            .clickable {
                                openFileWithDefaultApp(context, File(message.filePath))
                            }
                            .padding(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Box(
                            modifier = Modifier
                                .size(36.dp)
                                .clip(CircleShape)
                                .background(palette.primary.copy(alpha = 0.2f)),
                            contentAlignment = Alignment.Center
                        ) {
                            Icon(
                                imageVector = Icons.Default.InsertDriveFile,
                                contentDescription = null,
                                tint = palette.primary,
                                modifier = Modifier.size(20.dp)
                            )
                        }
                        Spacer(modifier = Modifier.width(10.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = message.fileName.ifBlank { "Документ" },
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.SemiBold,
                                color = TextPrimary,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                            if (message.fileSizeFormatted.isNotBlank()) {
                                Text(
                                    text = message.fileSizeFormatted,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextMuted
                                )
                            }
                        }
                    }
                } else {
                    Text(
                        text = message.text,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextPrimary,
                        fontSize = fontSizeSp.sp,
                        lineHeight = (fontSizeSp + 5).sp
                    )
                }

                Spacer(modifier = Modifier.height(2.dp))

                Row(
                    modifier = Modifier.align(Alignment.End),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = message.timestampStr,
                        style = MaterialTheme.typography.labelSmall,
                        color = if (isMe) TextPrimary.copy(alpha = 0.7f) else TextMuted,
                        fontSize = 10.sp
                    )
                    if (isMe) {
                        Spacer(modifier = Modifier.width(4.dp))
                        Icon(
                            imageVector = if (message.isDelivered) Icons.Default.DoneAll else Icons.Default.Schedule,
                            contentDescription = if (message.isDelivered) "Доставлено" else "Ожидает подтверждения доставки",
                            tint = if (message.isDelivered) NodeXEmerald else TextMuted,
                            modifier = Modifier.size(14.dp)
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun ContactInfoDialog(
    contactName: String,
    contactIdHex: String,
    onClearHistory: () -> Unit,
    onDeleteContact: () -> Unit,
    onDismiss: () -> Unit
) {
    val context = LocalContext.current

    Dialog(onDismissRequest = onDismiss) {
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(containerColor = NodeXSurface)
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "Информация о контакте",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    IconButton(onClick = onDismiss, modifier = Modifier.size(24.dp)) {
                        Icon(imageVector = Icons.Default.Close, contentDescription = null, tint = TextSecondary)
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(56.dp)
                            .clip(CircleShape)
                            .background(
                                Brush.linearGradient(
                                    listOf(Color(0xFF2F81F7), Color(0xFF1F4E79))
                                )
                            ),
                        contentAlignment = Alignment.Center
                    ) {
                        val initial = contactName.filter { it.isLetterOrDigit() }.take(1).ifEmpty { "U" }.uppercase()
                        Text(
                            text = initial,
                            color = Color.White,
                            fontWeight = FontWeight.Bold,
                            fontSize = 24.sp
                        )
                    }
                    Spacer(modifier = Modifier.width(14.dp))
                    Column {
                        Text(
                            text = contactName,
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                            color = TextPrimary
                        )
                        Text(
                            text = "Защищенный P2P контакт",
                            style = MaterialTheme.typography.bodySmall,
                            color = NodeXEmerald
                        )
                    }
                }

                Spacer(modifier = Modifier.height(18.dp))

                Text(text = "Node ID (публичный ключ)", style = MaterialTheme.typography.labelSmall, color = TextMuted)
                Spacer(modifier = Modifier.height(6.dp))
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(10.dp))
                        .background(NodeXBackground)
                        .padding(horizontal = 12.dp, vertical = 10.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = contactIdHex,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        color = TextPrimary,
                        modifier = Modifier.weight(1f)
                    )
                    IconButton(
                        onClick = {
                            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                            clipboard.setPrimaryClip(ClipData.newPlainText("Contact ID", contactIdHex))
                            Toast.makeText(context, "ID скопирован в буфер", Toast.LENGTH_SHORT).show()
                        },
                        modifier = Modifier.size(26.dp)
                    ) {
                        Icon(imageVector = Icons.Default.ContentCopy, contentDescription = null, tint = NodeXPrimary)
                    }
                }

                Spacer(modifier = Modifier.height(22.dp))

                Button(
                    onClick = onClearHistory,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXSurfaceVariant)
                ) {
                    Text("Очистить историю сообщений", color = TextPrimary)
                }

                Spacer(modifier = Modifier.height(8.dp))

                Button(
                    onClick = onDeleteContact,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXDestructive)
                ) {
                    Text("Удалить контакт", color = Color.White)
                }
            }
        }
    }
}

private fun openFileWithDefaultApp(context: Context, file: File, mimeType: String = "*/*") {
    try {
        if (!file.exists()) {
            Toast.makeText(context, "Файл не найден на устройстве", Toast.LENGTH_SHORT).show()
            return
        }
        val uri = FileProvider.getUriForFile(
            context,
            "network.nodex.messenger.fileprovider",
            file
        )
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, mimeType)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        context.startActivity(intent)
    } catch (e: Exception) {
        Toast.makeText(context, "Не найдено приложение для открытия файла", Toast.LENGTH_SHORT).show()
    }
}
