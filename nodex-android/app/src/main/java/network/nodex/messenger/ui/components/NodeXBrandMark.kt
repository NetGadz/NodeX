package network.nodex.messenger.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Compact vector rendering of the NodeX mark from the supplied reference. */
@Composable
fun NodeXBrandMark(
    modifier: Modifier = Modifier,
    size: Dp = 52.dp,
    accent: Color = Color(0xFF45C7FF),
    animated: Boolean = false
) {
    Canvas(modifier = modifier.size(size)) {
        val s = this.size.minDimension
        val left = s * .08f
        val top = s * .08f
        val right = s * .92f
        val bottom = s * .92f
        val radius = s * .27f
        drawRoundRect(
            color = Color(0xFF080B16),
            topLeft = Offset(left, top),
            size = androidx.compose.ui.geometry.Size(right - left, bottom - top),
            cornerRadius = androidx.compose.ui.geometry.CornerRadius(radius),
        )
        drawRoundRect(
            brush = Brush.linearGradient(listOf(Color(0xFF394A94), Color(0xFF171B38), Color(0xFF7044E8))),
            topLeft = Offset(left, top),
            size = androidx.compose.ui.geometry.Size(right - left, bottom - top),
            cornerRadius = androidx.compose.ui.geometry.CornerRadius(radius),
            style = Stroke(width = s * .012f)
        )

        // A continuous, rounded N ribbon, using the reference cyan-to-violet gradient.
        val ribbon = Path().apply {
            moveTo(s * .28f, s * .66f)
            lineTo(s * .28f, s * .36f)
            quadraticBezierTo(s * .28f, s * .28f, s * .35f, s * .36f)
            lineTo(s * .66f, s * .70f)
            quadraticBezierTo(s * .73f, s * .77f, s * .73f, s * .68f)
            lineTo(s * .73f, s * .37f)
        }
        drawPath(
            path = ribbon,
            brush = Brush.linearGradient(listOf(Color(0xFF62E4FF), accent, Color(0xFF5288FF), Color(0xFF9562FF), Color(0xFFE1A0FF))),
            style = Stroke(width = s * .14f, cap = StrokeCap.Round, join = StrokeJoin.Round)
        )
    }
}
