use super::fetch::Fetch;
use super::fifo::Fifo;
use super::pixel::Pixel;
use super::Ppu;

#[derive(Debug, Default)]
pub struct Pipeline {
    discard: u8,
    xpos: u8,
    bgwin: Channel,
    sprite: Channel,
}

impl Pipeline {
    #[must_use]
    pub fn xpos(&self) -> u8 {
        self.xpos
    }

    pub fn set_discard(&mut self, discard: u8) {
        self.discard = discard;
    }

    pub fn fetch(&mut self, ppu: &mut Ppu) {
        // FIXME: Clock the sprite fetcher as well
        self.bgwin.fetch(ppu)
    }

    pub fn shift(&mut self) -> Option<Pixel> {
        // A shift only occurs if there are pixels in the background FIFO
        let pixel = if let Some(bgwin) = self.bgwin.fifo.pop() {
            // Now also pop the sprite FIFO
            if let Some(sprite) = self.sprite.fifo.pop() {
                // Blend the two pixels together
                Pixel::blend(bgwin, sprite)
            } else {
                // No sprite, so use the background FIFO's pixel
                bgwin
            }
        } else {
            // Nothing in the background FIFO
            return None;
        };

        // Check if this pixel needs to be discarded (as a part of implementing
        // the behaviour of SCX)
        if self.discard > 0 {
            // One fewer pixel to discard
            self.discard -= 1;
            // Discard this pixel
            None
        } else {
            // Move internal x-position
            self.xpos += 1;
            // Return this pixel
            Some(pixel)
        }
    }
}

#[derive(Debug, Default)]
struct Channel {
    fetch: Fetch,
    fifo: Fifo,
}

impl Channel {
    pub fn fetch(&mut self, ppu: &mut Ppu) {
        self.fetch.exec(&mut self.fifo, ppu)
    }
}
