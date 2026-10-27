//! Software cursors composited into libretro video. The Ruffle player decides
//! which shape applies; these sprites never participate in hit testing.
use ruffle_core::backend::ui::MouseCursor;

// Original monochrome sprites: # = outline, o = white, space = transparent.
const CROSS: &[&str] = &[
    "    #o#",
    "    #o#",
    "    #o#",
    "    #o#",
    "#####o#####",
    "ooooooooooo",
    "#####o#####",
    "    #o#",
    "    #o#",
    "    #o#",
    "    #o#",
];
const HAND: &[&str] = &[
    "     ##",
    "    #oo#",
    "    #oo#",
    "    #oo#",
    "    #oo#",
    "    #oo###",
    "    #oo#oo###",
    "    #oo#oo#oo##",
    "    #oo#oo#oo#o#",
    " ## #oo#oo#oo#o#",
    "#oo##oooooooooo#",
    "#ooo#oooooooooo#",
    " #ooooooooooooo#",
    "  #oooooooooooo#",
    "  #oooooooooooo#",
    "   #oooooooooo#",
    "    #ooooooooo#",
    "    #oooooooo#",
    "     #oooooo#",
    "     ########",
];
pub fn draw(video: &mut [u32], width: u32, height: u32, x: u32, y: u32, shape: MouseCursor) {
    // Preserve the original cross everywhere except the pointing-hand state
    // Ruffle selects for buttons/links. A grab cursor is not a clickable link.
    let (sprite, hotspot) = match shape {
        MouseCursor::Hand => (HAND, (5, 0)),
        MouseCursor::Arrow | MouseCursor::IBeam | MouseCursor::Grab => (CROSS, (5, 5)),
    };
    for (row, pixels) in sprite.iter().enumerate() {
        for (column, pixel) in pixels.bytes().enumerate() {
            let color = match pixel {
                b'#' => 0,
                b'o' => 0xffffff,
                _ => continue,
            };
            let px = i64::from(x) + column as i64 - hotspot.0;
            let py = i64::from(y) + row as i64 - hotspot.1;
            if px >= 0 && py >= 0 && px < i64::from(width) && py < i64::from(height) {
                video[py as usize * width as usize + px as usize] = color;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprites_clip_at_every_edge_without_changing_buffer_size() {
        for shape in [
            MouseCursor::Arrow,
            MouseCursor::Hand,
            MouseCursor::IBeam,
            MouseCursor::Grab,
        ] {
            for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
                let mut pixels = vec![0x123456; 32 * 32];
                draw(&mut pixels, 32, 32, x, y, shape);
                assert_eq!(pixels.len(), 32 * 32);
                assert!(pixels.iter().any(|p| *p != 0x123456));
                assert_eq!(pixels[16 * 32 + 16], 0x123456);
            }
            draw(&mut [], 0, 0, 0, 0, shape);
        }
    }
}
