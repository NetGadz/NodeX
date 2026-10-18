package network.nodex.messenger.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import network.nodex.messenger.data.ChatTheme

@Composable
fun NodeXTheme(
    theme: ChatTheme = ChatTheme.GRAPHITE,
    customAccent: Color? = null,
    content: @Composable () -> Unit
) {
    val palette = getThemeColors(theme, customAccent)
    val colorScheme = darkColorScheme(
        primary = palette.primary,
        secondary = palette.primary,
        tertiary = NodeXEmerald,
        background = palette.background,
        surface = palette.surface,
        surfaceVariant = palette.surface,
        onPrimary = Color.White,
        onSecondary = Color.White,
        onBackground = palette.textPrimary,
        onSurface = palette.textPrimary
    )

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
