package network.nodex.messenger.ui.theme

import androidx.compose.ui.graphics.Color
import network.nodex.messenger.data.ChatTheme

val NodeXBackground = Color(0xFF090B12)
val NodeXSurface = Color(0xFF11131E)
val NodeXSurfaceVariant = Color(0xFF191B29)
val NodeXBorder = Color(0xFF282B3D)

val NodeXPrimary = Color(0xFF6D8CFF)
val NodeXBrandCyan = Color(0xFF56D9FF)
val NodeXBrandViolet = Color(0xFF985CFF)
val NodeXEmerald = Color(0xFF58D6A5)
val NodeXWarning = Color(0xFFFFAB00)
val NodeXDestructive = Color(0xFFEF4444)

val NodeXBubbleOut = Color(0xFF123F43)
val NodeXBubbleIn = Color(0xFF151D2B)

val TextPrimary = Color(0xFFF1F5F9)
val TextSecondary = Color(0xFF99A8B9)
val TextMuted = Color(0xFF657386)
val DividerColor = Color(0xFF202B3B)

// 9-Swatch Circular Palette (Telegram / NodeX Style)
val SwatchAzure = Color(0xFF007AFF)      // 1. Лазурный
val SwatchCyan = Color(0xFF00E5FF)       // 2. Неоновый циан
val SwatchEmerald = Color(0xFF00C853)    // 3. Изумруд
val SwatchViolet = Color(0xFF7C4DFF)     // 4. Фиолетовый
val SwatchPink = Color(0xFFFF4081)       // 5. Пурпурный
val SwatchAmber = Color(0xFFFFAB00)      // 6. Янтарь
val SwatchCrimson = Color(0xFFFF5252)    // 7. Рубин
val SwatchTeal = Color(0xFF00BFA5)       // 8. Бирюзовый
val SwatchSlate = Color(0xFF78909C)      // 9. Стелс

data class AccentSwatch(
    val color: Color,
    val hex: String,
    val title: String
)

val ACCENT_SWATCHES = listOf(
    AccentSwatch(SwatchAzure, "#007AFF", "Лазурный"),
    AccentSwatch(SwatchCyan, "#00E5FF", "Неоновый циан"),
    AccentSwatch(SwatchEmerald, "#00C853", "Изумруд"),
    AccentSwatch(SwatchViolet, "#7C4DFF", "Фиолетовый"),
    AccentSwatch(SwatchPink, "#FF4081", "Пурпурный"),
    AccentSwatch(SwatchAmber, "#FFAB00", "Янтарь"),
    AccentSwatch(SwatchCrimson, "#FF5252", "Рубин"),
    AccentSwatch(SwatchTeal, "#00BFA5", "Бирюзовый"),
    AccentSwatch(SwatchSlate, "#78909C", "Стелс")
)

fun getThemeColors(theme: ChatTheme, customAccent: Color? = null): ThemePalette {
    val effectivePrimary = customAccent ?: Color(theme.primaryHex)
    val effectiveBubbleOut = customAccent?.let {
        Color(
            red = it.red * 0.7f + 0.05f,
            green = it.green * 0.7f + 0.05f,
            blue = it.blue * 0.7f + 0.1f,
            alpha = 1f
        )
    } ?: Color(theme.bubbleOutHex)

    return ThemePalette(
        background = Color(theme.bgHex),
        surface = Color(theme.surfaceHex),
        surfaceVariant = NodeXSurfaceVariant,
        borderColor = NodeXBorder,
        primary = effectivePrimary,
        bubbleOut = effectiveBubbleOut,
        bubbleIn = Color(theme.bubbleInHex),
        textPrimary = TextPrimary,
        textSecondary = TextSecondary
    )
}

typealias ThemeColors = ThemePalette

data class ThemePalette(
    val background: Color,
    val surface: Color,
    val surfaceVariant: Color = NodeXSurfaceVariant,
    val borderColor: Color = NodeXBorder,
    val primary: Color,
    val bubbleOut: Color,
    val bubbleIn: Color,
    val textPrimary: Color,
    val textSecondary: Color
)
