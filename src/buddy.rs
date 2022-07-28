use super::math::{round_up_2, trailing_zero_right};

use std::alloc::{AllocError, Layout};
use std::ptr::NonNull;
use std::sync::Mutex;

const MIN_BUDDY_SIZE: usize = 16;
const MAX_BUDDY_SIZE: usize = 0x8000_0000;
const MAX_SUPPORTED_ALIGN: usize = 4096;

/// Buddy Allocator
pub struct BuddyAllocator<'a>(pub Mutex<ProtectedAllocator<'a>>);

impl<'a> BuddyAllocator<'a> {
    /// Create a new Buddy Allocator on a previous allocated block
    pub const fn new(address: &'a mut [u8]) -> Self {
        Self(Mutex::new(ProtectedAllocator::new(address)))
    }
}

// TODO. on final time, this struct must be placed into a choosen memory location
#[repr(C, align(4096))]
pub struct ProtectedAllocator<'a>(&'a mut [u8]);

impl<'a> ProtectedAllocator<'a> {
    const fn new(address: &'a mut [u8]) -> Self {
        assert!(address.len() <= MAX_BUDDY_SIZE); // Max limitation cf. 32b systems
        assert!(address.len() >= MIN_BUDDY_SIZE); // Min limitation cf. 32b systems
        let space_rounded_up_2 = round_up_2(address.len() as u32);
        let space_order0_buddy = if space_rounded_up_2 == address.len() as u32 {
            address.len() as u32
        } else {
            space_rounded_up_2 >> 1
        };
        let min_aligned = if address.len() > MAX_SUPPORTED_ALIGN {
            MAX_SUPPORTED_ALIGN
        } else {
            address.len()
        };
        let ptr_offset = address.as_mut_ptr().align_offset(min_aligned as usize);
        // On launch time with const fn feature, align_offset() doesn't works and returns USIZE::MAX
        // Trust on you
        assert!(ptr_offset == 0 || ptr_offset == usize::MAX); // Check pointer alignement
        Self(address.get_mut(0..space_order0_buddy as usize).unwrap())
    }
    #[inline(always)]
    fn set_mark(&mut self, order: Order) -> Result<NonNull<[u8]>, &'static str> {
        // Recurse descent into orders
        // if 0b00 | 0b01 go to the left
        // if 0b10 go the the right
        // if 0b00 and order is good, mark it
        // set 1 bit to parent
        // if 0b11 then recurse and set 0b11
        dbg!(order);
        Ok(NonNull::from(&self.0[..]))
    }
    #[inline(always)]
    fn unset_mark(&mut self, order: Order, ptr: NonNull<u8>) -> Result<(), &'static str> {
        // check ptr align with order
        // let shr = 0 + (1 << order.0) + ptr(order);
        // verify mask 0b10 % ptr offset
        // set mask 0b00
        // verify nex buddy if 0b10
        // if 0b10 get 0 + (1 << (order.0 - 1)) + ptr(offset -> set as 0b01)
        // Verify if 0b11
        // if 0b00 get 0 + (1 << (order.0 - 1)) + ptr(offset -> set as 0b00)
        // Verify if 0b10 or (0b01) ???
        // END
        dbg!(order);
        dbg!(ptr);
        Ok(())
    }
}

// Requested Buddy Size and Order with their TryFrom<_> boilerplates
#[derive(Debug)]
struct BuddySize(u32);
#[derive(Debug)]
struct Order(u32);

impl TryFrom<(BuddySize, BuddySize)> for Order {
    type Error = &'static str;
    fn try_from((buddy_size, max_buddy_size): (BuddySize, BuddySize)) -> Result<Self, Self::Error> {
        dbg!(&buddy_size);
        let buddy_pow = trailing_zero_right(buddy_size.0);
        let space_pow = trailing_zero_right(max_buddy_size.0);
        if buddy_pow > space_pow {
            Err("the bigger buddy is too small for the requested size")
        } else {
            Ok(Order(space_pow - buddy_pow))
        }
    }
}

// TODO: Put MAX_SUPPORTED_ALIGN & MAX_BUDDY_SIZE into static string
impl TryFrom<Layout> for BuddySize {
    type Error = &'static str;
    fn try_from(layout: Layout) -> Result<Self, Self::Error> {
        let size = usize::max(
            usize::max(layout.size(), layout.align()),
            MIN_BUDDY_SIZE as usize,
        );
        match u32::try_from(size) {
            Ok(size) => {
                if size as usize > MAX_BUDDY_SIZE {
                    Err("Size must be lower or eq than {MAX_BUDDY_SIZE}")
                } else if layout.align() > MAX_SUPPORTED_ALIGN {
                    Err("Alignement too big: MAX - {MAX_SUPPORTED_ALIGN}")
                } else {
                    Ok(BuddySize(round_up_2(size)))
                }
            }
            Err(_e) => Err("Requested size must be fit into an u32"),
        }
    }
}

fn format_error(e: &'static str) -> AllocError {
    eprintln!("{}", e);
    AllocError
}

impl<'a> ProtectedAllocator<'a> {
    #[inline(always)]
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        match BuddySize::try_from(layout) {
            Ok(buddy_size) => match Order::try_from((buddy_size, BuddySize(self.0.len() as u32))) {
                Ok(order) => self.set_mark(order).map_err(|e| format_error(e)),
                Err(e) => Err(format_error(e)),
            },
            Err(e) => Err(format_error(e)),
        }
    }
    #[inline(always)]
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        match BuddySize::try_from(layout) {
            Ok(buddy_size) => match Order::try_from((buddy_size, BuddySize(self.0.len() as u32))) {
                Ok(order) => self.unset_mark(order, ptr).unwrap(),
                Err(e) => panic!("{}", e),
            },
            Err(e) => panic!("{}", e),
        }
    }
}

impl<'a> Drop for BuddyAllocator<'a> {
    fn drop(&mut self) {}
}

impl<'a> Drop for ProtectedAllocator<'a> {
    fn drop(&mut self) {}
}