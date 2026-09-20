use egui::{Color32, Pos2, Rect, Stroke, Vec2};

/// Draw a vector paper plane (Telegram Send Icon)
pub fn draw_send_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = rect.width().min(rect.height()) * 0.55;
    
    // Triangle points for a sleek paper plane
    let tip = Pos2::new(center.x + size * 0.6, center.y);
    let top_left = Pos2::new(center.x - size * 0.6, center.y - size * 0.55);
    let bottom_left = Pos2::new(center.x - size * 0.6, center.y + size * 0.55);
    let middle_dent = Pos2::new(center.x - size * 0.2, center.y);

    painter.add(egui::Shape::convex_polygon(
        vec![top_left, tip, middle_dent],
        color,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![bottom_left, tip, middle_dent],
        Color32::from_rgba_premultiplied(
            color.r().saturating_sub(30),
            color.g().saturating_sub(30),
            color.b().saturating_sub(30),
            color.a(),
        ),
        Stroke::NONE,
    ));
}

/// Draw a vector microphone icon
pub fn draw_mic_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.22;
    
    // Mic capsule
    let capsule_rect = Rect::from_center_size(Pos2::new(center.x, center.y - r * 0.4), Vec2::new(r * 1.3, r * 2.2));
    painter.rect_filled(capsule_rect, egui::Rounding::same(r * 0.65), color);

    // Outer cradle arc
    let stroke = Stroke::new(1.8_f32, color);
    let cradle_top_l = Pos2::new(center.x - r * 1.1, center.y - r * 0.2);
    let cradle_bot = Pos2::new(center.x, center.y + r * 1.2);
    let cradle_top_r = Pos2::new(center.x + r * 1.1, center.y - r * 0.2);

    painter.line_segment([cradle_top_l, Pos2::new(center.x - r * 1.1, center.y + r * 0.6)], stroke);
    painter.line_segment([Pos2::new(center.x - r * 1.1, center.y + r * 0.6), cradle_bot], stroke);
    painter.line_segment([cradle_bot, Pos2::new(center.x + r * 1.1, center.y + r * 0.6)], stroke);
    painter.line_segment([Pos2::new(center.x + r * 1.1, center.y + r * 0.6), cradle_top_r], stroke);

    // Stem and base
    painter.line_segment([cradle_bot, Pos2::new(center.x, center.y + r * 1.8)], stroke);
    painter.line_segment([Pos2::new(center.x - r * 0.8, center.y + r * 1.8), Pos2::new(center.x + r * 0.8, center.y + r * 1.8)], stroke);
}

/// Draw a vector paperclip (attachment icon)
pub fn draw_paperclip_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let s = rect.width().min(rect.height()) * 0.35;
    let stroke = Stroke::new(1.8_f32, color);

    let p1 = Pos2::new(center.x - s * 0.5, center.y + s * 0.5);
    let p2 = Pos2::new(center.x + s * 0.4, center.y - s * 0.4);
    let p3 = Pos2::new(center.x + s * 0.7, center.y - s * 0.1);
    let p4 = Pos2::new(center.x - s * 0.2, center.y + s * 0.8);
    let p5 = Pos2::new(center.x - s * 0.7, center.y + s * 0.3);
    let p6 = Pos2::new(center.x + s * 0.1, center.y - s * 0.5);

    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p5], stroke);
    painter.line_segment([p5, p6], stroke);
}

/// Draw vector magnifying glass (Search icon)
pub fn draw_search_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.25;
    let stroke = Stroke::new(1.8_f32, color);

    let circle_center = Pos2::new(center.x - r * 0.3, center.y - r * 0.3);
    painter.circle_stroke(circle_center, r, stroke);

    let handle_start = Pos2::new(circle_center.x + r * 0.7, circle_center.y + r * 0.7);
    let handle_end = Pos2::new(center.x + r * 1.3, center.y + r * 1.3);
    painter.line_segment([handle_start, handle_end], Stroke::new(2.2_f32, color));
}

/// Draw vector gear (Settings icon)
pub fn draw_gear_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.32;
    let stroke = Stroke::new(2.0_f32, color);

    painter.circle_stroke(center, r * 0.6, stroke);

    // 6 cogs around perimeter
    for i in 0..6 {
        let angle = (i as f32) * std::f32::consts::PI / 3.0;
        let c = angle.cos();
        let s = angle.sin();
        let p_inner = Pos2::new(center.x + c * r * 0.6, center.y + s * r * 0.6);
        let p_outer = Pos2::new(center.x + c * r, center.y + s * r);
        painter.line_segment([p_inner, p_outer], Stroke::new(2.4_f32, color));
    }
}

/// Draw vector single checkmark
pub fn draw_single_check(painter: &egui::Painter, pos: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    let c1_a = Pos2::new(pos.x, pos.y + size * 0.5);
    let c1_b = Pos2::new(pos.x + size * 0.3, pos.y + size * 0.85);
    let c1_c = Pos2::new(pos.x + size * 0.8, pos.y + size * 0.15);
    painter.line_segment([c1_a, c1_b], stroke);
    painter.line_segment([c1_b, c1_c], stroke);
}

/// Draw vector double checkmark
pub fn draw_double_check(painter: &egui::Painter, pos: Pos2, size: f32, color: Color32) {
    let stroke = Stroke::new(1.5_f32, color);
    
    // First checkmark
    let c1_a = Pos2::new(pos.x, pos.y + size * 0.5);
    let c1_b = Pos2::new(pos.x + size * 0.3, pos.y + size * 0.85);
    let c1_c = Pos2::new(pos.x + size * 0.8, pos.y + size * 0.15);
    painter.line_segment([c1_a, c1_b], stroke);
    painter.line_segment([c1_b, c1_c], stroke);

    // Second overlapping checkmark
    let offset = size * 0.45;
    let c2_a = Pos2::new(pos.x + offset, pos.y + size * 0.5);
    let c2_b = Pos2::new(pos.x + size * 0.3 + offset, pos.y + size * 0.85);
    let c2_c = Pos2::new(pos.x + size * 0.8 + offset, pos.y + size * 0.15);
    painter.line_segment([c2_a, c2_b], stroke);
    painter.line_segment([c2_b, c2_c], stroke);
}

/// Draw a sharp vector play triangle
pub fn draw_play_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = rect.width().min(rect.height()) * 0.35;
    let p1 = Pos2::new(center.x - size * 0.4, center.y - size * 0.6);
    let p2 = Pos2::new(center.x + size * 0.6, center.y);
    let p3 = Pos2::new(center.x - size * 0.4, center.y + size * 0.6);
    painter.add(egui::Shape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
}

/// Draw sharp vector pause double bars
pub fn draw_pause_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let size = rect.width().min(rect.height()) * 0.32;
    let bar_w = 3.0_f32;
    let bar_h = size * 1.2;

    let b1 = Rect::from_center_size(Pos2::new(center.x - size * 0.4, center.y), Vec2::new(bar_w, bar_h));
    let b2 = Rect::from_center_size(Pos2::new(center.x + size * 0.4, center.y), Vec2::new(bar_w, bar_h));
    painter.rect_filled(b1, egui::Rounding::same(1.0), color);
    painter.rect_filled(b2, egui::Rounding::same(1.0), color);
}

pub const NODEX_LOGO_BYTES: &[u8] = include_bytes!("../../assets/photo.jpg");

/// Render the official NodeX photo logo from assets
pub fn render_nodex_logo_widget(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let img = egui::Image::from_bytes("bytes://nodex_logo_photo.jpg", NODEX_LOGO_BYTES)
        .fit_to_exact_size(Vec2::splat(size))
        .rounding(egui::Rounding::same(8.0));
    ui.add(img)
}

/// Fallback painter for vector logo
pub fn draw_nodex_logo(painter: &egui::Painter, rect: Rect, _accent: Color32) {
    let center = rect.center();
    let size = rect.width().min(rect.height());
    let r = size * 0.45;

    // Smooth rounded dark container
    let bg_color = Color32::from_rgb(19, 23, 34); // #131722
    let border_color = Color32::from_rgb(34, 41, 58); // #22293A
    painter.rect_filled(rect, egui::Rounding::same(8.0), bg_color);
    painter.rect_stroke(rect, egui::Rounding::same(8.0), Stroke::new(1.0_f32, border_color));

    // 3 Triad Node Centers matching the 3D 'N' organic loop
    let p_top = Pos2::new(center.x + r * 0.05, center.y - r * 0.45);
    let p_left = Pos2::new(center.x - r * 0.48, center.y + r * 0.42);
    let p_right = Pos2::new(center.x + r * 0.50, center.y + r * 0.42);

    let bridge_width = r * 0.28;
    let node_radius = r * 0.26;

    // Organic curved connecting bridges
    let steps = 16;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;

        let ctrl_left = Pos2::new(center.x - r * 0.25, center.y - r * 0.25);
        let pt0 = quadratic_bezier(p_left, ctrl_left, p_top, t0);
        let pt1 = quadratic_bezier(p_left, ctrl_left, p_top, t1);

        let w = bridge_width * (0.85 + (t0 - 0.5).abs() * 0.3);
        painter.line_segment([pt0, pt1], Stroke::new(w, Color32::from_rgb(230, 235, 245)));
    }

    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;

        let ctrl_right = Pos2::new(center.x + r * 0.35, center.y - r * 0.05);
        let pt0 = quadratic_bezier(p_top, ctrl_right, p_right, t0);
        let pt1 = quadratic_bezier(p_top, ctrl_right, p_right, t1);

        let w = bridge_width * (0.85 + (t0 - 0.5).abs() * 0.3);
        painter.line_segment([pt0, pt1], Stroke::new(w, Color32::from_rgb(215, 222, 235)));
    }

    draw_3d_sphere(painter, p_left, node_radius);
    draw_3d_sphere(painter, p_right, node_radius);
    draw_3d_sphere(painter, p_top, node_radius * 1.05);
}

fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let ut2 = 2.0 * u * t;

    Pos2::new(
        uu * p0.x + ut2 * p1.x + tt * p2.x,
        uu * p0.y + ut2 * p1.y + tt * p2.y,
    )
}

fn draw_3d_sphere(painter: &egui::Painter, center: Pos2, radius: f32) {
    // Base shadow / ambient
    painter.circle_filled(center, radius, Color32::from_rgb(200, 208, 222));
    
    // Smooth diffuse layer
    let diffuse_center = Pos2::new(center.x - radius * 0.15, center.y - radius * 0.18);
    painter.circle_filled(diffuse_center, radius * 0.85, Color32::from_rgb(238, 242, 250));

    // Specular highlight
    let spec_center = Pos2::new(center.x - radius * 0.28, center.y - radius * 0.32);
    painter.circle_filled(spec_center, radius * 0.38, Color32::WHITE);
}

/// Draw a crisp 5-pointed star icon (for Saved Messages avatar)
pub fn draw_star_bookmark_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r_outer = rect.width().min(rect.height()) * 0.38;
    let r_inner = r_outer * 0.40;
    let mut points = Vec::with_capacity(10);

    for i in 0..10 {
        let angle = (i as f32) * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { r_outer } else { r_inner };
        points.push(Pos2::new(center.x + angle.cos() * r, center.y + angle.sin() * r));
    }

    // Draw as 10 triangular sectors connected to center (strictly convex triangles)
    for i in 0..10 {
        let p1 = points[i];
        let p2 = points[(i + 1) % 10];
        painter.add(egui::Shape::convex_polygon(vec![center, p1, p2], color, Stroke::NONE));
    }
}

/// Draw a vector Info 'i' icon
#[allow(dead_code)]
pub fn draw_info_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.35;
    let stroke = Stroke::new(1.6_f32, color);
    painter.circle_stroke(center, r, stroke);

    // Dot of 'i'
    painter.circle_filled(Pos2::new(center.x, center.y - r * 0.45), 1.6, color);
    // Stem of 'i'
    painter.line_segment(
        [Pos2::new(center.x, center.y - r * 0.1), Pos2::new(center.x, center.y + r * 0.45)],
        Stroke::new(1.8_f32, color),
    );
}

/// Draw a vector Phone call handset icon
#[allow(dead_code)]
pub fn draw_phone_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let s = rect.width().min(rect.height()) * 0.35;
    let stroke = Stroke::new(2.0_f32, color);

    let p1 = Pos2::new(center.x - s * 0.5, center.y + s * 0.4);
    let p2 = Pos2::new(center.x - s * 0.3, center.y - s * 0.3);
    let p3 = Pos2::new(center.x + s * 0.4, center.y - s * 0.5);

    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.circle_filled(p1, 2.8, color);
    painter.circle_filled(p3, 2.8, color);
}

/// Draw a vector eye icon for reveal / hide seed
#[allow(dead_code)]
pub fn draw_eye_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
    let center = rect.center();
    let r = rect.width().min(rect.height()) * 0.32;
    let stroke = Stroke::new(1.8_f32, color);

    // Outer eye almond shape
    let p_left = Pos2::new(center.x - r * 1.2, center.y);
    let p_right = Pos2::new(center.x + r * 1.2, center.y);
    let p_top = Pos2::new(center.x, center.y - r * 0.7);
    let p_bot = Pos2::new(center.x, center.y + r * 0.7);

    painter.line_segment([p_left, p_top], stroke);
    painter.line_segment([p_top, p_right], stroke);
    painter.line_segment([p_right, p_bot], stroke);
    painter.line_segment([p_bot, p_left], stroke);

    // Pupil
    painter.circle_filled(center, r * 0.38, color);
}


