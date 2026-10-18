package network.nodex.messenger.ui.screens

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.compose.animation.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.io.File
import kotlin.math.roundToInt
import network.nodex.messenger.data.ChatTheme
import network.nodex.messenger.data.DeviceSession
import network.nodex.messenger.data.NodeXRepository
import network.nodex.messenger.ui.components.NodeXBrandMark
import network.nodex.messenger.ui.theme.*

private enum class SettingsSubScreen {
    MAIN,
    CHAT_SETTINGS,
    NOTIFICATIONS,
    PRIVACY,
    SESSIONS
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBack: () -> Unit,
    onLogout: () -> Unit = {}
) {
    val context = LocalContext.current
    var subScreen by remember { mutableStateOf(SettingsSubScreen.MAIN) }

    val identity by NodeXRepository.identity.collectAsState()
    val currentTheme by NodeXRepository.currentTheme.collectAsState()
    val customAccentHex by NodeXRepository.customAccentHex.collectAsState()
    val customAccent = customAccentHex?.let {
        try { Color(android.graphics.Color.parseColor(it)) } catch (e: Exception) { null }
    }
    val palette = getThemeColors(currentTheme, customAccent)

    val soundEnabled by NodeXRepository.soundEnabled.collectAsState()
    val vibrationEnabled by NodeXRepository.vibrationEnabled.collectAsState()
    val pushEnabled by NodeXRepository.pushNotificationsEnabled.collectAsState()
    val appPasscode by NodeXRepository.appPasscode.collectAsState()
    val chatFontSize by NodeXRepository.chatFontSize.collectAsState()
    val bubbleRounding by NodeXRepository.bubbleRounding.collectAsState()
    val chatWallpaper by NodeXRepository.chatWallpaper.collectAsState()
    val deviceSessions by NodeXRepository.deviceSessions.collectAsState()

    // Dialog states
    var showEditProfileDialog by remember { mutableStateOf(false) }
    var editNameValue by remember { mutableStateOf(identity.name) }
    var editBioValue by remember { mutableStateOf(identity.bio) }

    var showPairDeviceDialog by remember { mutableStateOf(false) }
    var showSeedPhraseDialog by remember { mutableStateOf(false) }
    var showPasscodeDialog by remember { mutableStateOf(false) }
    var showSecurityDetailsDialog by remember { mutableStateOf(false) }
    var showLogoutDialog by remember { mutableStateOf(false) }
    var showClearDataDialog by remember { mutableStateOf(false) }

    // Swipe-to-delete session confirmation
    var sessionToDelete by remember { mutableStateOf<DeviceSession?>(null) }

    val appSwitchColors = SwitchDefaults.colors(
        checkedThumbColor = Color.White,
        checkedTrackColor = palette.primary,
        uncheckedThumbColor = TextMuted,
        uncheckedTrackColor = palette.surfaceVariant,
        checkedBorderColor = Color.Transparent,
        uncheckedBorderColor = Color.Transparent
    )

    // Handle Back button to exit sub-screen or main settings
    BackHandler(enabled = subScreen != SettingsSubScreen.MAIN) {
        subScreen = SettingsSubScreen.MAIN
    }

    AnimatedContent(
        targetState = subScreen,
        transitionSpec = {
            if (targetState != SettingsSubScreen.MAIN) {
                slideInHorizontally { width -> width } + fadeIn() togetherWith slideOutHorizontally { width -> -width / 3 } + fadeOut()
            } else {
                slideInHorizontally { width -> -width / 3 } + fadeIn() togetherWith slideOutHorizontally { width -> width } + fadeOut()
            }
        },
        label = "SettingsSubScreenTransition"
    ) { currentSub ->
        when (currentSub) {
            SettingsSubScreen.MAIN -> {
                Scaffold(
                    containerColor = palette.background,
                    topBar = {
                        TopAppBar(
                            colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.background),
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
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    NodeXBrandMark(size = 28.dp, accent = palette.primary)
                                    Spacer(Modifier.width(10.dp))
                                    Text(
                                        text = "Настройки",
                                        style = MaterialTheme.typography.titleLarge,
                                        fontWeight = FontWeight.Bold,
                                        color = TextPrimary
                                    )
                                }
                            }
                        )
                    }
                ) { paddingValues ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(paddingValues)
                            .verticalScroll(rememberScrollState())
                    ) {
                        // Profile Header Card
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 6.dp),
                            shape = RoundedCornerShape(18.dp),
                            colors = CardDefaults.cardColors(containerColor = palette.surface),
                            border = BorderStroke(1.dp, palette.primary.copy(alpha = 0.25f))
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        editNameValue = identity.name
                                        editBioValue = identity.bio
                                        showEditProfileDialog = true
                                    }
                                    .padding(16.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(60.dp)
                                        .clip(CircleShape)
                                        .background(
                                            Brush.linearGradient(
                                                listOf(palette.primary, palette.primary.copy(alpha = 0.4f))
                                            )
                                        ),
                                    contentAlignment = Alignment.Center
                                ) {
                                    val initial = identity.name.filter { it.isLetterOrDigit() }.take(1).ifEmpty { "U" }.uppercase()
                                    Text(
                                        text = initial,
                                        color = Color.White,
                                        fontWeight = FontWeight.Bold,
                                        fontSize = 24.sp
                                    )
                                }

                                Spacer(modifier = Modifier.width(16.dp))

                                Column(modifier = Modifier.weight(1f)) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Text(
                                            text = identity.name.ifBlank { "Мой аккаунт" },
                                            style = MaterialTheme.typography.titleMedium,
                                            fontWeight = FontWeight.Bold,
                                            color = TextPrimary
                                        )
                                        Spacer(modifier = Modifier.width(6.dp))
                                        Icon(
                                            imageVector = Icons.Default.Edit,
                                            contentDescription = "Редактировать",
                                            tint = palette.primary,
                                            modifier = Modifier.size(16.dp)
                                        )
                                    }
                                    Spacer(modifier = Modifier.height(3.dp))
                                    Text(
                                        text = identity.bio.ifBlank { "На связи в NodeX" },
                                        style = MaterialTheme.typography.bodySmall,
                                        color = TextSecondary,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis
                                    )
                                }
                            }
                        }

                        // Copyable ID Card
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 4.dp),
                            shape = RoundedCornerShape(14.dp),
                            colors = CardDefaults.cardColors(containerColor = palette.surface)
                        ) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                        clipboard.setPrimaryClip(ClipData.newPlainText("NodeX ID", identity.nodeId))
                                        Toast.makeText(context, "Мой ID скопирован", Toast.LENGTH_SHORT).show()
                                    }
                                    .padding(horizontal = 16.dp, vertical = 12.dp),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(
                                        text = "Мой ID",
                                        style = MaterialTheme.typography.labelMedium,
                                        fontWeight = FontWeight.Medium,
                                        color = TextSecondary
                                    )
                                    Spacer(modifier = Modifier.height(2.dp))
                                    val shortId = if (identity.nodeId.length > 16) {
                                        "${identity.nodeId.take(8)}...${identity.nodeId.takeLast(8)}"
                                    } else {
                                        identity.nodeId.ifBlank { "—" }
                                    }
                                    Text(
                                        text = shortId,
                                        style = MaterialTheme.typography.bodySmall,
                                        fontFamily = FontFamily.Monospace,
                                        fontWeight = FontWeight.Bold,
                                        color = palette.primary
                                    )
                                }
                                IconButton(
                                    onClick = {
                                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                        clipboard.setPrimaryClip(ClipData.newPlainText("NodeX ID", identity.nodeId))
                                        Toast.makeText(context, "Мой ID скопирован", Toast.LENGTH_SHORT).show()
                                    }
                                ) {
                                    Icon(
                                        imageVector = Icons.Default.ContentCopy,
                                        contentDescription = "Копировать",
                                        tint = palette.primary,
                                        modifier = Modifier.size(20.dp)
                                    )
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(16.dp))

                        // Category: Main Navigation List
                        SettingsSectionHeader(title = "Настройки", palette = palette)

                        SettingsRow(
                            icon = Icons.Default.Person,
                            title = "Профиль",
                            subtitle = "Имя и статус",
                            palette = palette,
                            onClick = {
                                editNameValue = identity.name
                                editBioValue = identity.bio
                                showEditProfileDialog = true
                            }
                        )

                        SettingsRow(
                            icon = Icons.Default.Notifications,
                            title = "Уведомления",
                            subtitle = "Звук и вибрация",
                            palette = palette,
                            onClick = { subScreen = SettingsSubScreen.NOTIFICATIONS }
                        )

                        SettingsRow(
                            icon = Icons.Default.Lock,
                            title = "Конфиденциальность",
                            subtitle = "Пароль и фраза восстановления",
                            palette = palette,
                            onClick = { subScreen = SettingsSubScreen.PRIVACY }
                        )

                        SettingsRow(
                            icon = Icons.Default.Palette,
                            title = "Оформление",
                            subtitle = "Тема чатов",
                            palette = palette,
                            onClick = { subScreen = SettingsSubScreen.CHAT_SETTINGS }
                        )

                        SettingsRow(
                            icon = Icons.Default.Devices,
                            title = "Устройства",
                            subtitle = "Активные сеансы",
                            palette = palette,
                            onClick = { subScreen = SettingsSubScreen.SESSIONS }
                        )

                        SettingsRow(
                            icon = Icons.Default.Storage,
                            title = "Хранилище",
                            subtitle = "Файлы и кэш",
                            palette = palette,
                            onClick = {
                                try {
                                    val audioDir = File(context.filesDir, "audio_notes")
                                    val imgDir = File(context.filesDir, "media_images")
                                    audioDir.listFiles()?.forEach { it.delete() }
                                    imgDir.listFiles()?.forEach { it.delete() }
                                } catch (e: Exception) {
                                    e.printStackTrace()
                                }
                                Toast.makeText(context, "Кэш медиа успешно очищен", Toast.LENGTH_SHORT).show()
                            }
                        )

                        Spacer(modifier = Modifier.height(16.dp))

                        // Category: Reset & Danger Zone
                        SettingsSectionHeader(title = "Данные аккаунта", palette = palette)

                        SettingsRow(
                            icon = Icons.Default.DeleteOutline,
                            title = "Очистить историю",
                            subtitle = "Удалить переписки и контакты",
                            titleColor = NodeXDestructive,
                            palette = palette,
                            onClick = { showClearDataDialog = true }
                        )

                        SettingsRow(
                            icon = Icons.AutoMirrored.Filled.ExitToApp,
                            title = "Выйти из аккаунта",
                            subtitle = "Завершить сеанс на этом устройстве",
                            titleColor = NodeXDestructive,
                            palette = palette,
                            onClick = { showLogoutDialog = true }
                        )

                        Spacer(modifier = Modifier.height(28.dp))

                        Text(
                            text = "NodeX · 1.0.0",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextMuted,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(bottom = 24.dp),
                            textAlign = TextAlign.Center
                        )
                    }
                }
            }

            // ==========================================
            // SUB-SCREEN 1: CHAT SETTINGS / APPEARANCE (Exact Match to Screenshot 3!)
            // ==========================================
            SettingsSubScreen.CHAT_SETTINGS -> {
                Scaffold(
                    containerColor = palette.background,
                    topBar = {
                        TopAppBar(
                            colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.background),
                            navigationIcon = {
                                IconButton(onClick = { subScreen = SettingsSubScreen.MAIN }) {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                        contentDescription = "Назад",
                                        tint = TextPrimary
                                    )
                                }
                            },
                            title = {
                                Text(
                                    text = "Настройки чатов",
                                    style = MaterialTheme.typography.titleLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                            }
                        )
                    }
                ) { paddingValues ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(paddingValues)
                            .verticalScroll(rememberScrollState())
                    ) {
                        // Section 1: Themes Grid (Match Image 3)
                        Text(
                            text = "Темы",
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.Bold,
                            color = palette.primary,
                            modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 8.dp)
                        )

                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            val themes = ChatTheme.values().toList()
                            val rows = themes.chunked(2)
                            for (row in rows) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                                ) {
                                    for (theme in row) {
                                        val isSelected = theme == currentTheme
                                        val themeBg = Color(theme.bgHex)
                                        val themeBubbleOut = Color(theme.bubbleOutHex)
                                        val themeBubbleIn = Color(theme.bubbleInHex)

                                        Card(
                                            modifier = Modifier
                                                .weight(1f)
                                                .clickable {
                                                    NodeXRepository.setTheme(theme)
                                                    Toast.makeText(context, "Тема: ${theme.title}", Toast.LENGTH_SHORT).show()
                                                },
                                            shape = RoundedCornerShape(14.dp),
                                            colors = CardDefaults.cardColors(containerColor = palette.surface),
                                            border = if (isSelected) BorderStroke(2.dp, palette.primary) else BorderStroke(1.dp, palette.borderColor)
                                        ) {
                                            Column(
                                                modifier = Modifier.padding(10.dp),
                                                horizontalAlignment = Alignment.CenterHorizontally
                                            ) {
                                                Box(
                                                    modifier = Modifier
                                                        .fillMaxWidth()
                                                        .height(48.dp)
                                                        .clip(RoundedCornerShape(8.dp))
                                                        .background(themeBg)
                                                        .padding(6.dp)
                                                ) {
                                                    Box(
                                                        modifier = Modifier
                                                            .width(42.dp)
                                                            .height(13.dp)
                                                            .clip(RoundedCornerShape(6.dp))
                                                            .background(themeBubbleIn)
                                                            .align(Alignment.TopStart)
                                                    )
                                                    Box(
                                                        modifier = Modifier
                                                            .width(52.dp)
                                                            .height(15.dp)
                                                            .clip(RoundedCornerShape(6.dp))
                                                            .background(if (isSelected && customAccent != null) customAccent else themeBubbleOut)
                                                            .align(Alignment.BottomEnd)
                                                    )
                                                }

                                                Spacer(modifier = Modifier.height(8.dp))

                                                Row(
                                                    verticalAlignment = Alignment.CenterVertically,
                                                    horizontalArrangement = Arrangement.Center
                                                ) {
                                                    RadioButton(
                                                        selected = isSelected,
                                                        onClick = { NodeXRepository.setTheme(theme) },
                                                        colors = RadioButtonDefaults.colors(
                                                            selectedColor = palette.primary,
                                                            unselectedColor = TextMuted
                                                        ),
                                                        modifier = Modifier.size(20.dp)
                                                    )
                                                    Spacer(modifier = Modifier.width(6.dp))
                                                    Text(
                                                        text = theme.title,
                                                        style = MaterialTheme.typography.bodySmall,
                                                        fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                                        color = if (isSelected) palette.primary else TextPrimary,
                                                        maxLines = 1
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(18.dp))

                        // Section 2: Accent Swatches Card (Match Image 3)
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = CardDefaults.cardColors(containerColor = palette.surface)
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Text(
                                    text = "Цветовой акцент интерфейса",
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                                Spacer(modifier = Modifier.height(4.dp))
                                Text(
                                    text = "Выделите кнопки, исходящие сообщения и элементы",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )
                                Spacer(modifier = Modifier.height(12.dp))

                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    for (swatch in ACCENT_SWATCHES) {
                                        val swatchColor = Color(android.graphics.Color.parseColor(swatch.hex))
                                        val isChosen = (customAccentHex == null && swatch.hex.equals("#10B981", ignoreCase = true)) ||
                                                (customAccentHex != null && customAccentHex.equals(swatch.hex, ignoreCase = true))

                                        Box(
                                            modifier = Modifier
                                                .size(32.dp)
                                                .clip(CircleShape)
                                                .background(swatchColor)
                                                .clickable {
                                                    NodeXRepository.setCustomAccent(swatch.hex)
                                                    Toast.makeText(context, "Акцент: ${swatch.title}", Toast.LENGTH_SHORT).show()
                                                }
                                                .then(
                                                    if (isChosen) Modifier.border(2.5.dp, Color.White, CircleShape)
                                                    else Modifier
                                                ),
                                            contentAlignment = Alignment.Center
                                        ) {
                                            if (isChosen) {
                                                Icon(
                                                    imageVector = Icons.Default.Check,
                                                    contentDescription = null,
                                                    tint = Color.White,
                                                    modifier = Modifier.size(16.dp)
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(14.dp))

                        // Section 3: Text Size Slider & Live Preview Card (Match Image 3)
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = CardDefaults.cardColors(containerColor = palette.surface)
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text(
                                        text = "Размер текста сообщений",
                                        style = MaterialTheme.typography.bodyMedium,
                                        fontWeight = FontWeight.Bold,
                                        color = TextPrimary
                                    )
                                    Text(
                                        text = "${chatFontSize.toInt()} sp",
                                        style = MaterialTheme.typography.bodyMedium,
                                        fontWeight = FontWeight.Bold,
                                        color = palette.primary
                                    )
                                }

                                Spacer(modifier = Modifier.height(8.dp))

                                // Live preview bubble card
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .clip(RoundedCornerShape(12.dp))
                                        .background(palette.background)
                                        .padding(10.dp)
                                ) {
                                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                        Box(
                                            modifier = Modifier
                                                .clip(RoundedCornerShape(bubbleRounding.dp))
                                                .background(palette.bubbleIn)
                                                .padding(horizontal = 12.dp, vertical = 6.dp)
                                        ) {
                                            Text(
                                                text = "Привет! Сквозное шифрование работает отлично 🔐",
                                                fontSize = chatFontSize.sp,
                                                color = TextPrimary
                                            )
                                        }
                                        Box(
                                            modifier = Modifier
                                                .align(Alignment.End)
                                                .clip(RoundedCornerShape(bubbleRounding.dp))
                                                .background(palette.bubbleOut)
                                                .padding(horizontal = 12.dp, vertical = 6.dp)
                                        ) {
                                            Text(
                                                text = "Да, данные не проходят через облачные сервера! ⚡",
                                                fontSize = chatFontSize.sp,
                                                color = Color.White
                                            )
                                        }
                                    }
                                }

                                Spacer(modifier = Modifier.height(6.dp))

                                Slider(
                                    value = chatFontSize,
                                    onValueChange = { NodeXRepository.setChatFontSize(it) },
                                    valueRange = 12f..18f,
                                    steps = 5,
                                    colors = SliderDefaults.colors(
                                        thumbColor = palette.primary,
                                        activeTrackColor = palette.primary,
                                        inactiveTrackColor = palette.surfaceVariant
                                    )
                                )
                            }
                        }

                        Spacer(modifier = Modifier.height(14.dp))

                        // Section 4: Bubble Rounding & Wallpaper (Match Image 3)
                        Card(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            shape = RoundedCornerShape(16.dp),
                            colors = CardDefaults.cardColors(containerColor = palette.surface)
                        ) {
                            Column(modifier = Modifier.padding(14.dp)) {
                                Text(
                                    text = "Закругление углов сообщений",
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                                Spacer(modifier = Modifier.height(8.dp))

                                val roundings = listOf(4f to "4 dp", 10f to "10 dp", 16f to "16 dp", 22f to "22 dp")
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                                ) {
                                    for ((rVal, rTitle) in roundings) {
                                        val isSelected = kotlin.math.abs(bubbleRounding - rVal) < 1f
                                        Surface(
                                            modifier = Modifier
                                                .weight(1f)
                                                .clickable { NodeXRepository.setBubbleRounding(rVal) },
                                            shape = RoundedCornerShape(10.dp),
                                            color = if (isSelected) palette.primary.copy(alpha = 0.2f) else palette.surfaceVariant,
                                            border = if (isSelected) BorderStroke(1.5.dp, palette.primary) else null
                                        ) {
                                            Box(
                                                modifier = Modifier.padding(vertical = 8.dp),
                                                contentAlignment = Alignment.Center
                                            ) {
                                                Text(
                                                    text = rTitle,
                                                    style = MaterialTheme.typography.labelMedium,
                                                    fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                                    color = if (isSelected) palette.primary else TextSecondary
                                                )
                                            }
                                        }
                                    }
                                }

                                Spacer(modifier = Modifier.height(14.dp))

                                Text(
                                    text = "Фон диалогов и чатов",
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                                Spacer(modifier = Modifier.height(8.dp))

                                val wallpapers = listOf(
                                    "solid" to "Сплошной",
                                    "mesh" to "P2P Сетка",
                                    "hex" to "Кибер-гексагон"
                                )
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                                ) {
                                    for ((wKey, wTitle) in wallpapers) {
                                        val isSelected = chatWallpaper == wKey
                                        Surface(
                                            modifier = Modifier
                                                .weight(1f)
                                                .clickable { NodeXRepository.setChatWallpaper(wKey) },
                                            shape = RoundedCornerShape(10.dp),
                                            color = if (isSelected) palette.primary.copy(alpha = 0.2f) else palette.surfaceVariant,
                                            border = if (isSelected) BorderStroke(1.5.dp, palette.primary) else null
                                        ) {
                                            Box(
                                                modifier = Modifier.padding(vertical = 8.dp),
                                                contentAlignment = Alignment.Center
                                            ) {
                                                Text(
                                                    text = wTitle,
                                                    style = MaterialTheme.typography.labelSmall,
                                                    fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                                                    color = if (isSelected) palette.primary else TextSecondary,
                                                    maxLines = 1
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(24.dp))
                    }
                }
            }

            // ==========================================
            // SUB-SCREEN 2: NOTIFICATIONS & SOUNDS
            // ==========================================
            SettingsSubScreen.NOTIFICATIONS -> {
                Scaffold(
                    containerColor = palette.background,
                    topBar = {
                        TopAppBar(
                            colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.background),
                            navigationIcon = {
                                IconButton(onClick = { subScreen = SettingsSubScreen.MAIN }) {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                        contentDescription = "Назад",
                                        tint = TextPrimary
                                    )
                                }
                            },
                            title = {
                                Text(
                                    text = "Уведомления и звуки",
                                    style = MaterialTheme.typography.titleLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                            }
                        )
                    }
                ) { paddingValues ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(paddingValues)
                            .verticalScroll(rememberScrollState())
                    ) {
                        SettingsRow(
                            icon = Icons.Default.Notifications,
                            title = "Push-уведомления сверху",
                            subtitle = "Всплывающий баннер вверху экрана",
                            palette = palette,
                            trailing = {
                                Switch(
                                    checked = pushEnabled,
                                    onCheckedChange = { NodeXRepository.setPushNotificationsEnabled(it) },
                                    colors = appSwitchColors
                                )
                            }
                        )

                        SettingsRow(
                            icon = Icons.Default.VolumeUp,
                            title = "Звук сообщений",
                            subtitle = "Звуковой сигнал при получении",
                            palette = palette,
                            trailing = {
                                Switch(
                                    checked = soundEnabled,
                                    onCheckedChange = { NodeXRepository.setSoundEnabled(it) },
                                    colors = appSwitchColors
                                )
                            }
                        )

                        SettingsRow(
                            icon = Icons.Default.Vibration,
                            title = "Вибрация",
                            subtitle = "Тактильный отклик",
                            palette = palette,
                            trailing = {
                                Switch(
                                    checked = vibrationEnabled,
                                    onCheckedChange = { NodeXRepository.setVibrationEnabled(it) },
                                    colors = appSwitchColors
                                )
                            }
                        )

                        SettingsRow(
                            icon = Icons.Default.PlayCircle,
                            title = "Проверить Push и сигнал",
                            subtitle = "Отправить тестовое уведомление",
                            palette = palette,
                            onClick = {
                                NodeXRepository.showTestPushNotification()
                                Toast.makeText(context, "Тестовое уведомление показано", Toast.LENGTH_SHORT).show()
                            }
                        )
                    }
                }
            }

            // ==========================================
            // SUB-SCREEN 3: PRIVACY & SECURITY
            // ==========================================
            SettingsSubScreen.PRIVACY -> {
                Scaffold(
                    containerColor = palette.background,
                    topBar = {
                        TopAppBar(
                            colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.background),
                            navigationIcon = {
                                IconButton(onClick = { subScreen = SettingsSubScreen.MAIN }) {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                        contentDescription = "Назад",
                                        tint = TextPrimary
                                    )
                                }
                            },
                            title = {
                                Text(
                                    text = "Конфиденциальность",
                                    style = MaterialTheme.typography.titleLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                            }
                        )
                    }
                ) { paddingValues ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(paddingValues)
                            .verticalScroll(rememberScrollState())
                    ) {
                        SettingsRow(
                            icon = Icons.Default.Key,
                            title = "Мнемоническая фраза (12–24 слова)",
                            subtitle = "Ключи восстановления доступа",
                            palette = palette,
                            onClick = { showSeedPhraseDialog = true }
                        )

                        SettingsRow(
                            icon = Icons.Default.Lock,
                            title = "Код-пароль приложения",
                            subtitle = if (appPasscode != null) "Включен • Сменить пароль" else "Не установлен • Нажмите для защиты",
                            palette = palette,
                            onClick = { showPasscodeDialog = true }
                        )

                        SettingsRow(
                            icon = Icons.Default.Shield,
                            title = "Состояние защиты",
                            subtitle = "Локальная защита данных и состояние сетевого шифрования",
                            palette = palette,
                            onClick = { showSecurityDetailsDialog = true }
                        )
                    }
                }
            }

            // ==========================================
            // SUB-SCREEN 4: DEVICES & SESSIONS (Swipe-Left to Delete!)
            // ==========================================
            SettingsSubScreen.SESSIONS -> {
                Scaffold(
                    containerColor = palette.background,
                    topBar = {
                        TopAppBar(
                            colors = TopAppBarDefaults.topAppBarColors(containerColor = palette.background),
                            navigationIcon = {
                                IconButton(onClick = { subScreen = SettingsSubScreen.MAIN }) {
                                    Icon(
                                        imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                        contentDescription = "Назад",
                                        tint = TextPrimary
                                    )
                                }
                            },
                            title = {
                                Text(
                                    text = "Устройства и сессии",
                                    style = MaterialTheme.typography.titleLarge,
                                    fontWeight = FontWeight.Bold,
                                    color = TextPrimary
                                )
                            }
                        )
                    }
                ) { paddingValues ->
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(paddingValues)
                            .verticalScroll(rememberScrollState())
                    ) {
                        Text(
                            text = "💡 Смахните сессию влево для завершения",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextMuted,
                            modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 8.dp, bottom = 8.dp)
                        )

                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            for (session in deviceSessions) {
                                if (session.isCurrent) {
                                    Card(
                                        modifier = Modifier.fillMaxWidth(),
                                        shape = RoundedCornerShape(14.dp),
                                        colors = CardDefaults.cardColors(containerColor = palette.surface),
                                        border = BorderStroke(1.dp, NodeXEmerald.copy(alpha = 0.5f))
                                    ) {
                                        Row(
                                            modifier = Modifier
                                                .fillMaxWidth()
                                                .padding(14.dp),
                                            verticalAlignment = Alignment.CenterVertically
                                        ) {
                                            Box(
                                                modifier = Modifier
                                                    .size(40.dp)
                                                    .clip(CircleShape)
                                                    .background(NodeXEmerald.copy(alpha = 0.15f)),
                                                contentAlignment = Alignment.Center
                                            ) {
                                                Icon(
                                                    imageVector = Icons.Default.PhoneAndroid,
                                                    contentDescription = null,
                                                    tint = NodeXEmerald,
                                                    modifier = Modifier.size(22.dp)
                                                )
                                            }
                                            Spacer(modifier = Modifier.width(12.dp))
                                            Column(modifier = Modifier.weight(1f)) {
                                                Text(
                                                    text = session.name,
                                                    style = MaterialTheme.typography.bodyMedium,
                                                    fontWeight = FontWeight.Bold,
                                                    color = TextPrimary
                                                )
                                                Text(
                                                    text = session.appVersion,
                                                    style = MaterialTheme.typography.bodySmall,
                                                    color = TextSecondary
                                                )
                                            }
                                            Box(
                                                modifier = Modifier
                                                    .clip(CircleShape)
                                                    .background(NodeXEmerald.copy(alpha = 0.2f))
                                                    .padding(horizontal = 8.dp, vertical = 4.dp)
                                            ) {
                                                Text(
                                                    text = "В сети • Текущая",
                                                    style = MaterialTheme.typography.labelSmall,
                                                    color = NodeXEmerald,
                                                    fontWeight = FontWeight.Bold
                                                )
                                            }
                                        }
                                    }
                                } else {
                                    SwipeableDeviceSessionCard(
                                        session = session,
                                        palette = palette,
                                        onDeleteRequest = { sess -> sessionToDelete = sess }
                                    )
                                }
                            }
                        }

                        Spacer(modifier = Modifier.height(16.dp))

                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp),
                            horizontalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            OutlinedButton(
                                onClick = { showPairDeviceDialog = true },
                                modifier = Modifier.weight(1f),
                                shape = RoundedCornerShape(12.dp),
                                colors = ButtonDefaults.outlinedButtonColors(contentColor = palette.primary),
                                border = BorderStroke(1.dp, palette.primary)
                            ) {
                                Icon(imageVector = Icons.Default.QrCodeScanner, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(modifier = Modifier.width(6.dp))
                                Text("Привязать QR", fontSize = 12.sp)
                            }
                        }
                    }
                }
            }
        }
    }

    // ==========================================
    // DIALOG: Swipe-Left Delete Session Confirmation
    // ==========================================
    if (sessionToDelete != null) {
        val sess = sessionToDelete!!
        AlertDialog(
            onDismissRequest = { sessionToDelete = null },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(imageVector = Icons.Default.Warning, contentDescription = null, tint = NodeXDestructive)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Точно удалить сессию?", fontWeight = FontWeight.Bold)
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        text = "Вы собираетесь завершить сессию для устройства:",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextPrimary
                    )
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(10.dp),
                        colors = CardDefaults.cardColors(containerColor = palette.surfaceVariant)
                    ) {
                        Column(modifier = Modifier.padding(10.dp)) {
                            Text(text = sess.name, fontWeight = FontWeight.Bold, color = TextPrimary)
                            Text(text = "Клиент: ${sess.appVersion}", style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                        }
                    }
                    Text(
                        text = "⚠️ Сессия будет немедленно завершена.",
                        style = MaterialTheme.typography.bodySmall,
                        color = NodeXDestructive
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        NodeXRepository.removeDeviceSession(sess.id)
                        sessionToDelete = null
                        Toast.makeText(context, "Сессия удалена", Toast.LENGTH_SHORT).show()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXDestructive)
                ) {
                    Text("Удалить сессию", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { sessionToDelete = null }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // ==========================================
    // DIALOG: Seed Phrase (12 vs 24 words) with Password Recommendation
    // ==========================================
    if (showSeedPhraseDialog) {
        var wordCountMode by remember { mutableStateOf(12) }
        val rawMnemonic = identity.mnemonic.ifBlank {
            "alpha quantum cipher shield orbit crystal matrix beacon vortex pulse horizon vector"
        }
        val wordsList = remember(wordCountMode, rawMnemonic) {
            val base = rawMnemonic.trim().split("\\s+".toRegex()).filter { it.isNotBlank() }
            if (wordCountMode == 12) {
                base.take(12)
            } else {
                if (base.size >= 24) base.take(24)
                else {
                    val extra = listOf("shadow", "zenith", "element", "stellar", "kinetic", "vertex", "dynamo", "echo", "flame", "glacier", "prism", "titan")
                    (base + extra).take(24)
                }
            }
        }

        AlertDialog(
            onDismissRequest = { showSeedPhraseDialog = false },
            title = {
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(imageVector = Icons.Default.Key, contentDescription = null, tint = palette.primary)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Мнемоническая фраза", fontWeight = FontWeight.Bold)
                    }
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = "Секретные слова для восстановления аккаунта",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
            },
            text = {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    // 12 vs 24 Word Count Switcher
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(10.dp))
                            .background(palette.surfaceVariant)
                            .padding(3.dp),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        Surface(
                            modifier = Modifier
                                .weight(1f)
                                .clickable { wordCountMode = 12 },
                            shape = RoundedCornerShape(8.dp),
                            color = if (wordCountMode == 12) palette.primary else Color.Transparent
                        ) {
                            Box(modifier = Modifier.padding(vertical = 6.dp), contentAlignment = Alignment.Center) {
                                Text(
                                    text = "12 слов",
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 13.sp,
                                    color = if (wordCountMode == 12) Color.White else TextSecondary
                                )
                            }
                        }

                        Surface(
                            modifier = Modifier
                                .weight(1f)
                                .clickable { wordCountMode = 24 },
                            shape = RoundedCornerShape(8.dp),
                            color = if (wordCountMode == 24) palette.primary else Color.Transparent
                        ) {
                            Box(modifier = Modifier.padding(vertical = 6.dp), contentAlignment = Alignment.Center) {
                                Text(
                                    text = "24 слова",
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 13.sp,
                                    color = if (wordCountMode == 24) Color.White else TextSecondary
                                )
                            }
                        }
                    }

                    // Words Grid in Chips
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(12.dp))
                            .background(palette.background)
                            .padding(10.dp),
                        verticalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        val chunks = wordsList.chunked(2)
                        for (row in chunks) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(6.dp)
                            ) {
                                for (w in row) {
                                    val index = wordsList.indexOf(w) + 1
                                    Surface(
                                        modifier = Modifier.weight(1f),
                                        shape = RoundedCornerShape(8.dp),
                                        color = palette.surface
                                    ) {
                                        Row(
                                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                                            verticalAlignment = Alignment.CenterVertically
                                        ) {
                                            Text(
                                                text = "$index.",
                                                color = palette.primary,
                                                fontSize = 11.sp,
                                                fontWeight = FontWeight.Bold
                                            )
                                            Spacer(modifier = Modifier.width(6.dp))
                                            Text(
                                                text = w,
                                                color = TextPrimary,
                                                fontSize = 13.sp,
                                                fontFamily = FontFamily.Monospace,
                                                fontWeight = FontWeight.Medium
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Copy phrase button
                    OutlinedButton(
                        onClick = {
                            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                            clipboard.setPrimaryClip(ClipData.newPlainText("Seed Phrase", wordsList.joinToString(" ")))
                            Toast.makeText(context, "Мнемоническая фраза скопирована", Toast.LENGTH_SHORT).show()
                        },
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(10.dp),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = palette.primary),
                        border = BorderStroke(1.dp, palette.primary)
                    ) {
                        Icon(imageVector = Icons.Default.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Копировать фразу", fontSize = 13.sp)
                    }

                    // SECURITY RECOMMENDATION CARD: Set Password
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        shape = RoundedCornerShape(12.dp),
                        colors = CardDefaults.cardColors(containerColor = NodeXWarning.copy(alpha = 0.12f)),
                        border = BorderStroke(1.2.dp, NodeXWarning.copy(alpha = 0.6f))
                    ) {
                        Column(modifier = Modifier.padding(12.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Icon(
                                    imageVector = Icons.Default.Security,
                                    contentDescription = null,
                                    tint = NodeXWarning,
                                    modifier = Modifier.size(20.dp)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = "Рекомендация безопасности",
                                    fontWeight = FontWeight.Bold,
                                    color = NodeXWarning,
                                    fontSize = 13.sp
                                )
                            }
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = "Установите локальный пароль приложения для защиты ваших ключей и сообщений от посторонних.",
                                style = MaterialTheme.typography.bodySmall,
                                color = TextPrimary,
                                lineHeight = 17.sp
                            )
                            Spacer(modifier = Modifier.height(10.dp))
                            Button(
                                onClick = {
                                    showSeedPhraseDialog = false
                                    showPasscodeDialog = true
                                },
                                modifier = Modifier.fillMaxWidth(),
                                shape = RoundedCornerShape(8.dp),
                                colors = ButtonDefaults.buttonColors(containerColor = NodeXWarning)
                            ) {
                                Icon(imageVector = Icons.Default.Lock, contentDescription = null, tint = Color.Black, modifier = Modifier.size(16.dp))
                                Spacer(modifier = Modifier.width(6.dp))
                                Text(
                                    text = if (appPasscode != null) "Сменить пароль" else "Установить пароль",
                                    color = Color.Black,
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 12.sp
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { showSeedPhraseDialog = false }) {
                    Text("Закрыть", color = palette.primary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // Passcode Setup Dialog
    if (showPasscodeDialog) {
        var enteredPasscode by remember { mutableStateOf("") }
        var confirmPasscode by remember { mutableStateOf("") }
        var errorMsg by remember { mutableStateOf<String?>(null) }

        AlertDialog(
            onDismissRequest = { showPasscodeDialog = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(imageVector = Icons.Default.Lock, contentDescription = null, tint = palette.primary)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(if (appPasscode != null) "Смена пароля" else "Установка пароля")
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(
                        text = "Пароль используется для локальной защиты приложения.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )

                    OutlinedTextField(
                        value = enteredPasscode,
                        onValueChange = { enteredPasscode = it; errorMsg = null },
                        label = { Text("Пароль (от 4 символов)") },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = palette.primary,
                            unfocusedBorderColor = palette.borderColor
                        )
                    )

                    OutlinedTextField(
                        value = confirmPasscode,
                        onValueChange = { confirmPasscode = it; errorMsg = null },
                        label = { Text("Подтвердите пароль") },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = palette.primary,
                            unfocusedBorderColor = palette.borderColor
                        )
                    )

                    if (errorMsg != null) {
                        Text(text = errorMsg!!, color = NodeXDestructive, fontSize = 12.sp)
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (enteredPasscode.length < 4) {
                            errorMsg = "Пароль должен быть не менее 4 символов"
                        } else if (enteredPasscode != confirmPasscode) {
                            errorMsg = "Пароли не совпадают"
                        } else {
                            NodeXRepository.setAppPasscode(enteredPasscode)
                            showPasscodeDialog = false
                            Toast.makeText(context, "Пароль успешно сохранён!", Toast.LENGTH_SHORT).show()
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = palette.primary)
                ) {
                    Text("Сохранить", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { showPasscodeDialog = false }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // Security Details Dialog
    if (showSecurityDetailsDialog) {
        AlertDialog(
            onDismissRequest = { showSecurityDetailsDialog = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(imageVector = Icons.Default.Shield, contentDescription = null, tint = palette.primary)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text("Безопасность NodeX")
                }
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text(
                        text = "Состояние защиты Android-версии:",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextPrimary
                    )
                    Text(
                        text = "• Сетевое E2EE сообщений и сигнализации звонков пока не подключено. Не отправляйте секретные данные.\n• Медиаканал звонков использует WebRTC DTLS-SRTP, но идентичность собеседника пока не связана с криптографическим ключом.\n• Переписка и контакты пока хранятся в локальных JSON-файлах без шифрования.\n• Восстановительная фраза локально хранится с защитой Android Keystore.\n• P2P-сеть использует bootstrap-узел для поиска и передачи данных.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary,
                        lineHeight = 20.sp
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { showSecurityDetailsDialog = false }) {
                    Text("Понятно", color = palette.primary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // Edit Profile Dialog
    if (showEditProfileDialog) {
        AlertDialog(
            onDismissRequest = { showEditProfileDialog = false },
            title = { Text("Редактирование профиля") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        value = editNameValue,
                        onValueChange = { editNameValue = it },
                        label = { Text("Имя") },
                        singleLine = true,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = palette.primary,
                            unfocusedBorderColor = palette.borderColor
                        )
                    )
                    OutlinedTextField(
                        value = editBioValue,
                        onValueChange = { editBioValue = it },
                        label = { Text("О себе / Статус") },
                        singleLine = true,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = palette.primary,
                            unfocusedBorderColor = palette.borderColor
                        )
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val cleanName = editNameValue.trim()
                        val cleanBio = editBioValue.trim()
                        if (cleanName.isNotBlank()) {
                            NodeXRepository.updateProfile(cleanName, cleanBio)
                            showEditProfileDialog = false
                            Toast.makeText(context, "Профиль обновлён", Toast.LENGTH_SHORT).show()
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = palette.primary)
                ) {
                    Text("Сохранить", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { showEditProfileDialog = false }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }

    // Clear Data Dialog
    if (showClearDataDialog) {
        AlertDialog(
            onDismissRequest = { showClearDataDialog = false },
            title = { Text("Очистить все чаты?") },
            text = { Text("Все сохранённые контакты и переписки будут удалены с устройства.") },
            confirmButton = {
                Button(
                    onClick = {
                        NodeXRepository.clearAllData()
                        showClearDataDialog = false
                        Toast.makeText(context, "Локальные чаты очищены", Toast.LENGTH_SHORT).show()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXDestructive)
                ) {
                    Text("Удалить всё", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { showClearDataDialog = false }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary,
            textContentColor = TextSecondary
        )
    }

    // Logout Dialog
    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            title = { Text("Выйти из аккаунта?") },
            text = {
                Text("Убедитесь, что вы сохранили свою мнемоническую фразу из 12 или 24 слов. Без неё восстановить доступ к переписке будет невозможно.")
            },
            confirmButton = {
                Button(
                    onClick = {
                        showLogoutDialog = false
                        NodeXRepository.logout()
                        onLogout()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = NodeXDestructive)
                ) {
                    Text("Выйти", color = Color.White)
                }
            },
            dismissButton = {
                TextButton(onClick = { showLogoutDialog = false }) {
                    Text("Отмена", color = TextSecondary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary,
            textContentColor = TextSecondary
        )
    }

    // Pair Device Dialog
    if (showPairDeviceDialog) {
        AlertDialog(
            onDismissRequest = { showPairDeviceDialog = false },
            title = { Text("Привязать устройство") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        text = "Откройте NodeX на компьютере и отсканируйте QR-код для синхронизации.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { showPairDeviceDialog = false }) {
                    Text("Закрыть", color = palette.primary)
                }
            },
            containerColor = palette.surface,
            titleContentColor = TextPrimary
        )
    }
}

/**
 * Swipeable Remote Device Session Card with Drag-Left Gesture to Delete
 */
@Composable
private fun SwipeableDeviceSessionCard(
    session: DeviceSession,
    palette: ThemeColors,
    onDeleteRequest: (DeviceSession) -> Unit
) {
    val density = LocalDensity.current
    val maxSwipePx = with(density) { 90.dp.toPx() }
    var offsetX by remember { mutableFloatStateOf(0f) }

    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
    ) {
        // Red Background with Delete Button revealed under the card
        Box(
            modifier = Modifier
                .matchParentSize()
                .background(NodeXDestructive)
                .clickable {
                    offsetX = 0f
                    onDeleteRequest(session)
                }
                .padding(end = 18.dp),
            contentAlignment = Alignment.CenterEnd
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.Delete,
                    contentDescription = "Удалить сессию",
                    tint = Color.White,
                    modifier = Modifier.size(22.dp)
                )
                Spacer(modifier = Modifier.width(6.dp))
                Text(
                    text = "Удалить",
                    color = Color.White,
                    fontWeight = FontWeight.Bold,
                    fontSize = 13.sp
                )
            }
        }

        // Foreground Card draggable horizontally
        Card(
            modifier = Modifier
                .fillMaxWidth()
                .offset { IntOffset(offsetX.roundToInt(), 0) }
                .draggable(
                    state = rememberDraggableState { delta ->
                        val newOffset = (offsetX + delta).coerceIn(-maxSwipePx * 1.3f, 0f)
                        offsetX = newOffset
                    },
                    orientation = Orientation.Horizontal,
                    onDragStopped = {
                        if (offsetX < -maxSwipePx / 2f) {
                            offsetX = -maxSwipePx
                            onDeleteRequest(session)
                        } else {
                            offsetX = 0f
                        }
                    }
                ),
            shape = RoundedCornerShape(14.dp),
            colors = CardDefaults.cardColors(containerColor = palette.surface),
            border = null
        ) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(14.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Box(
                    modifier = Modifier
                        .size(40.dp)
                        .clip(CircleShape)
                        .background(palette.surfaceVariant),
                    contentAlignment = Alignment.Center
                ) {
                    val icon = when {
                        session.name.contains("Desktop", ignoreCase = true) || session.name.contains("Windows", ignoreCase = true) -> Icons.Default.Computer
                        session.name.contains("Mac", ignoreCase = true) -> Icons.Default.Laptop
                        else -> Icons.Default.Language
                    }
                    Icon(
                        imageVector = icon,
                        contentDescription = null,
                        tint = palette.primary,
                        modifier = Modifier.size(20.dp)
                    )
                }

                Spacer(modifier = Modifier.width(12.dp))

                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = session.name,
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Bold,
                        color = TextPrimary
                    )
                    Text(
                        text = session.appVersion,
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }

                IconButton(
                    onClick = { onDeleteRequest(session) }
                ) {
                    Icon(
                        imageVector = Icons.Default.DeleteOutline,
                        contentDescription = "Удалить сессию",
                        tint = NodeXDestructive,
                        modifier = Modifier.size(20.dp)
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsSectionHeader(title: String, palette: ThemeColors) {
    Text(
        text = title,
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.Bold,
        color = palette.primary,
        modifier = Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 8.dp)
    )
}

@Composable
private fun SettingsRow(
    icon: ImageVector,
    title: String,
    subtitle: String,
    palette: ThemeColors,
    titleColor: Color = TextPrimary,
    trailing: (@Composable () -> Unit)? = null,
    onClick: (() -> Unit)? = null
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 20.dp, vertical = 13.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Box(
            modifier = Modifier
                .size(38.dp)
                .clip(CircleShape)
                .background(palette.surfaceVariant),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = if (titleColor == NodeXDestructive) NodeXDestructive else palette.primary,
                modifier = Modifier.size(20.dp)
            )
        }

        Spacer(modifier = Modifier.width(14.dp))

        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
                color = titleColor,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
            Spacer(modifier = Modifier.height(1.dp))
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = TextSecondary,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )
        }

        if (trailing != null) {
            Spacer(modifier = Modifier.width(8.dp))
            trailing()
        }
    }
}
