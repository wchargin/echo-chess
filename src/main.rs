struct Pawn;
struct Bishop;
struct Rook;
struct Monarch;

/// A subset of the squares on a chess board.
///
/// Square in file `x` and rank `y` is indicated by bit `8 * y + x`. For instance, B1 is bit `1`
/// and A2 is bit `8`.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct SquareSet(u64);

impl SquareSet {
    fn draw(self: SquareSet) -> String {
        let mut res = String::new();
        for y in (0..8).rev() {
            res.push(char::from_u32('a' as u32 + y).unwrap());
            res.push(' ');
            for x in 0..8 {
                if self.0 & (1 << (8 * y + x)) != 0 {
                    res.push('*');
                } else {
                    res.push('.');
                }
            }
            res.push('\n');
        }
        res.push_str("  12345678\n");
        res
    }
}

impl std::ops::BitAnd<SquareSet> for SquareSet {
    type Output = SquareSet;
    fn bitand(self, rhs: SquareSet) -> SquareSet {
        SquareSet(self.0 & rhs.0)
    }
}
impl std::ops::BitOr<SquareSet> for SquareSet {
    type Output = SquareSet;
    fn bitor(self, rhs: SquareSet) -> SquareSet {
        SquareSet(self.0 | rhs.0)
    }
}
impl std::ops::Not for SquareSet {
    type Output = SquareSet;
    fn not(self) -> SquareSet {
        SquareSet(!self.0)
    }
}
impl std::ops::Shl<u32> for SquareSet {
    type Output = SquareSet;
    fn shl(self, rhs: u32) -> SquareSet {
        SquareSet(self.0 << rhs)
    }
}
impl std::ops::Shr<u32> for SquareSet {
    type Output = SquareSet;
    fn shr(self, rhs: u32) -> SquareSet {
        SquareSet(self.0 >> rhs)
    }
}

trait Stepper {
    fn move_steps(from: SquareSet) -> SquareSet;
    fn capture_steps(from: SquareSet) -> SquareSet;
}

fn captures<S: Stepper>(from: SquareSet, obstacles: SquareSet, targets: SquareSet) -> SquareSet {
    let permeable = !(obstacles | targets);
    let mut reachable = from & permeable;
    loop {
        let next = (reachable | S::move_steps(reachable)) & permeable;
        if next == reachable {
            break;
        }
        reachable = next;
    }
    S::capture_steps(reachable) & targets
}

mod can_move {
    use super::SquareSet;

    pub(crate) const LEFT: SquareSet = SquareSet(!0x0101010101010101);
    pub(crate) const RIGHT: SquareSet = SquareSet(!0x8080808080808080);
}

impl Stepper for Pawn {
    fn move_steps(from: SquareSet) -> SquareSet {
        SquareSet(from.0 << 8)
    }
    fn capture_steps(from: SquareSet) -> SquareSet {
        ((from & can_move::LEFT) << 7) | ((from & can_move::RIGHT) << 9)
    }
}

impl Stepper for Bishop {
    fn move_steps(from: SquareSet) -> SquareSet {
        let left = from & can_move::LEFT;
        let right = from & can_move::RIGHT;
        (left >> 9) | (right >> 7) | (left << 7) | (right << 9)
    }
    fn capture_steps(from: SquareSet) -> SquareSet {
        Self::move_steps(from)
    }
}

impl Stepper for Rook {
    fn move_steps(from: SquareSet) -> SquareSet {
        (from >> 8) | ((from & can_move::LEFT) >> 1) | ((from & can_move::RIGHT) << 1) | (from << 8)
    }
    fn capture_steps(from: SquareSet) -> SquareSet {
        Self::move_steps(from)
    }
}

impl Stepper for Monarch {
    fn move_steps(from: SquareSet) -> SquareSet {
        Rook::move_steps(from) | Bishop::move_steps(from)
    }
    fn capture_steps(from: SquareSet) -> SquareSet {
        Self::move_steps(from)
    }
}

fn main() {
    let start = SquareSet(0x8040201008040201);
    println!("start:\n{}", start.draw());
    println!("pawn steps:\n{}", Pawn::move_steps(start).draw());
    println!("pawn captures:\n{}", Pawn::capture_steps(start).draw());

    let start = SquareSet(0x4000_0010_0000_0001);
    println!("start:\n{}", start.draw());
    println!("bishop steps:\n{}", Bishop::move_steps(start).draw());
    println!("rook steps:\n{}", Rook::move_steps(start).draw());
    println!("monarch steps:\n{}", Monarch::move_steps(start).draw());
}
