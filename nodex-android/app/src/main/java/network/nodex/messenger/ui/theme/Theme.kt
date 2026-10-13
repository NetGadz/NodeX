package network.nodex.messenger.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val DarkColorScheme = darkColorScheme(
    primary = NodeXCyan,
    secondary = NodeXPurple,
    tertiary = NodeXEmerald,
    background = NodeXBackground,
    surface = NodeXSurface,
    onPrimary = NodeXBackground,
    onSecondary = TextPrimary,
    onBackground = TextPrimary,
    onSurface = TextPrimary
)

@Composable
fun NodeXTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = DarkColorScheme,
        typography = Typography,
        content = content
    )
}
