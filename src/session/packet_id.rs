//! Packet identifier allocation and tracking

use std::collections::HashSet;

/// Packet identifier allocator per MQTT 3.1.1 §2.3.1
///
/// Packet identifiers are 16-bit values (1-65535, 0 is invalid).
/// Each direction (client→broker, broker→client) has independent ID spaces.
/// IDs can be reused after the associated message flow is complete.
#[derive(Debug)]
pub struct PacketIdAllocator {
    /// Currently in-use packet IDs
    in_use: HashSet<u16>,
    /// Next ID to try allocating
    next_id: u16,
}

impl Default for PacketIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PacketIdAllocator {
    /// Create a new packet ID allocator
    pub fn new() -> Self {
        PacketIdAllocator {
            in_use: HashSet::new(),
            next_id: 1,
        }
    }

    /// Allocate a new packet ID.
    /// Returns None if all 65535 IDs are in use (shouldn't happen normally).
    pub fn allocate(&mut self) -> Option<u16> {
        // Quick path: next_id is available
        if !self.in_use.contains(&self.next_id) {
            let id = self.next_id;
            self.in_use.insert(id);
            self.next_id = if self.next_id == u16::MAX { 1 } else { self.next_id + 1 };
            return Some(id);
        }

        // Slow path: scan for available ID
        let start = self.next_id;
        loop {
            self.next_id = if self.next_id == u16::MAX { 1 } else { self.next_id + 1 };

            if !self.in_use.contains(&self.next_id) {
                let id = self.next_id;
                self.in_use.insert(id);
                self.next_id = if self.next_id == u16::MAX { 1 } else { self.next_id + 1 };
                return Some(id);
            }

            // Wrapped around without finding available ID
            if self.next_id == start {
                return None;
            }
        }
    }

    /// Release a packet ID for reuse
    pub fn release(&mut self, id: u16) {
        self.in_use.remove(&id);
    }

    /// Check if an ID is currently in use
    pub fn is_in_use(&self, id: u16) -> bool {
        self.in_use.contains(&id)
    }

    /// Mark an ID as in use (for incoming messages)
    pub fn mark_in_use(&mut self, id: u16) -> bool {
        self.in_use.insert(id)
    }

    /// Get count of IDs currently in use
    pub fn in_use_count(&self) -> usize {
        self.in_use.len()
    }

    /// Check if allocator has capacity
    pub fn has_capacity(&self) -> bool {
        self.in_use.len() < 65535
    }

    /// Clear all allocations (for session cleanup)
    pub fn clear(&mut self) {
        self.in_use.clear();
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_sequential() {
        let mut alloc = PacketIdAllocator::new();

        assert_eq!(alloc.allocate(), Some(1));
        assert_eq!(alloc.allocate(), Some(2));
        assert_eq!(alloc.allocate(), Some(3));

        assert!(alloc.is_in_use(1));
        assert!(alloc.is_in_use(2));
        assert!(alloc.is_in_use(3));
        assert!(!alloc.is_in_use(4));
    }

    #[test]
    fn test_release_and_reuse() {
        let mut alloc = PacketIdAllocator::new();

        let id1 = alloc.allocate().unwrap();
        let id2 = alloc.allocate().unwrap();

        alloc.release(id1);
        assert!(!alloc.is_in_use(id1));
        assert!(alloc.is_in_use(id2));

        // Should continue with next sequential ID, not reuse id1 immediately
        let id3 = alloc.allocate().unwrap();
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_wraparound() {
        let mut alloc = PacketIdAllocator::new();
        alloc.next_id = u16::MAX;

        let id1 = alloc.allocate().unwrap();
        assert_eq!(id1, u16::MAX);

        let id2 = alloc.allocate().unwrap();
        assert_eq!(id2, 1); // Wrapped to 1
    }

    #[test]
    fn test_exhaustion() {
        let mut alloc = PacketIdAllocator::new();

        // Fill up all IDs
        for _ in 0..65535 {
            assert!(alloc.allocate().is_some());
        }

        // Should fail
        assert!(alloc.allocate().is_none());
        assert!(!alloc.has_capacity());

        // Release one
        alloc.release(1000);
        assert!(alloc.has_capacity());

        // Should succeed now
        assert_eq!(alloc.allocate(), Some(1000));
    }

    #[test]
    fn test_clear() {
        let mut alloc = PacketIdAllocator::new();

        alloc.allocate();
        alloc.allocate();
        alloc.allocate();

        assert_eq!(alloc.in_use_count(), 3);

        alloc.clear();

        assert_eq!(alloc.in_use_count(), 0);
        assert_eq!(alloc.allocate(), Some(1));
    }
}
