use glam::IVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMap<T> {
    tiles: HashMap<IVec2, T>,
    tile_size: u32,
}

impl<T> TileMap<T> {
    pub fn new(tile_size: u32) -> Self {
        assert!(tile_size > 0, "tile_size must be positive");
        Self {
            tiles: HashMap::new(),
            tile_size,
        }
    }

    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    pub fn get(&self, coord: IVec2) -> Option<&T> {
        self.tiles.get(&coord)
    }

    pub fn get_mut(&mut self, coord: IVec2) -> Option<&mut T> {
        self.tiles.get_mut(&coord)
    }

    pub fn insert(&mut self, coord: IVec2, tile: T) -> Option<T> {
        self.tiles.insert(coord, tile)
    }

    pub fn remove(&mut self, coord: IVec2) -> Option<T> {
        self.tiles.remove(&coord)
    }

    pub fn contains(&self, coord: IVec2) -> bool {
        self.tiles.contains_key(&coord)
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn coords(&self) -> impl Iterator<Item = &IVec2> {
        self.tiles.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&IVec2, &T)> {
        self.tiles.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&IVec2, &mut T)> {
        self.tiles.iter_mut()
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
    }

    pub fn pixel_to_tile(&self, pixel_x: i32, pixel_y: i32) -> IVec2 {
        let ts = self.tile_size as i32;
        IVec2::new(
            pixel_x.div_euclid(ts),
            pixel_y.div_euclid(ts),
        )
    }

    pub fn tile_to_pixel_origin(&self, coord: IVec2) -> IVec2 {
        let ts = self.tile_size as i32;
        IVec2::new(coord.x * ts, coord.y * ts)
    }

    pub fn tiles_in_rect(&self, min_pixel: IVec2, max_pixel: IVec2) -> Vec<IVec2> {
        let min_tile = self.pixel_to_tile(min_pixel.x, min_pixel.y);
        let max_tile = self.pixel_to_tile(max_pixel.x, max_pixel.y);
        let mut result = Vec::new();
        for y in min_tile.y..=max_tile.y {
            for x in min_tile.x..=max_tile.x {
                result.push(IVec2::new(x, y));
            }
        }
        result
    }
}

impl<T> TileMap<T>
where
    T: Default,
{
    pub fn get_or_create(&mut self, coord: IVec2) -> &mut T {
        self.tiles.entry(coord).or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tile_map_is_empty() {
        let map: TileMap<Vec<u8>> = TileMap::new(256);
        assert!(map.is_empty());
        assert_eq!(map.tile_count(), 0);
        assert_eq!(map.tile_size(), 256);
    }

    #[test]
    fn insert_and_get() {
        let mut map = TileMap::new(64);
        map.insert(IVec2::new(0, 0), vec![1u8, 2, 3]);
        assert!(map.contains(IVec2::new(0, 0)));
        assert_eq!(map.get(IVec2::new(0, 0)).unwrap(), &vec![1u8, 2, 3]);
    }

    #[test]
    fn remove_tile() {
        let mut map = TileMap::new(64);
        map.insert(IVec2::new(1, 2), 42u32);
        assert_eq!(map.remove(IVec2::new(1, 2)), Some(42));
        assert!(!map.contains(IVec2::new(1, 2)));
    }

    #[test]
    fn pixel_to_tile_positive() {
        let map: TileMap<()> = TileMap::new(256);
        assert_eq!(map.pixel_to_tile(0, 0), IVec2::new(0, 0));
        assert_eq!(map.pixel_to_tile(255, 255), IVec2::new(0, 0));
        assert_eq!(map.pixel_to_tile(256, 0), IVec2::new(1, 0));
        assert_eq!(map.pixel_to_tile(512, 512), IVec2::new(2, 2));
    }

    #[test]
    fn pixel_to_tile_negative() {
        let map: TileMap<()> = TileMap::new(256);
        assert_eq!(map.pixel_to_tile(-1, -1), IVec2::new(-1, -1));
        assert_eq!(map.pixel_to_tile(-256, 0), IVec2::new(-1, 0));
        assert_eq!(map.pixel_to_tile(-257, 0), IVec2::new(-2, 0));
    }

    #[test]
    fn tile_to_pixel_origin() {
        let map: TileMap<()> = TileMap::new(256);
        assert_eq!(map.tile_to_pixel_origin(IVec2::new(0, 0)), IVec2::new(0, 0));
        assert_eq!(map.tile_to_pixel_origin(IVec2::new(1, 2)), IVec2::new(256, 512));
        assert_eq!(map.tile_to_pixel_origin(IVec2::new(-1, -1)), IVec2::new(-256, -256));
    }

    #[test]
    fn tiles_in_rect() {
        let map: TileMap<()> = TileMap::new(100);
        let tiles = map.tiles_in_rect(IVec2::new(50, 50), IVec2::new(250, 150));
        assert_eq!(tiles.len(), 6); // 3 columns x 2 rows
    }

    #[test]
    fn get_or_create_default() {
        let mut map: TileMap<Vec<u8>> = TileMap::new(64);
        let tile = map.get_or_create(IVec2::new(0, 0));
        assert!(tile.is_empty());
        tile.push(42);
        assert_eq!(map.get(IVec2::new(0, 0)).unwrap(), &vec![42u8]);
    }

    #[test]
    #[should_panic(expected = "tile_size must be positive")]
    fn zero_tile_size_panics() {
        let _: TileMap<()> = TileMap::new(0);
    }
}
