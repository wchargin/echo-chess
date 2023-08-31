use std::collections::{HashMap, HashSet};

struct Pawn;
struct Bishop;
struct Rook;
struct Monarch;
struct Knight;

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
    pub(crate) const TWO_LEFT: SquareSet = SquareSet(!0x0303030303030303);
    pub(crate) const TWO_RIGHT: SquareSet = SquareSet(!0xc0c0c0c0c0c0c0c0);
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

impl Stepper for Knight {
    fn move_steps(from: SquareSet) -> SquareSet {
        let left1 = from & can_move::LEFT;
        let left2 = from & can_move::TWO_LEFT;
        let right1 = from & can_move::RIGHT;
        let right2 = from & can_move::TWO_RIGHT;
        (left1 >> 17)
            | (right1 >> 15)
            | (left2 >> 10)
            | (right2 >> 6)
            | (left2 << 6)
            | (right2 << 10)
            | (left1 << 15)
            | (right1 << 17)
    }
    fn capture_steps(from: SquareSet) -> SquareSet {
        Self::move_steps(from)
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
enum PieceType {
    Pawn,
    Bishop,
    Rook,
    Monarch,
    Knight,
}

#[derive(Debug, Clone)]
struct Puzzle {
    obstacles: SquareSet,
    piece_types: [Option<PieceType>; 32],
    piece_locs: [u8; 32], // maps piece index (0..27) to board square (0..64) or 0xff
    pieces_by_loc: [u8; 64], // maps board square (0..64) to piece index (0..27) or 0xff
    player_start: u32,
}

/// Bits 0 through 26 (inclusive) indicate which pieces still need to be captured. The integer
/// formed by bits 27 through 31 (i.e., the value of `z >> 27`) indicates which piece is currently
/// the player.
///
/// Thus, this type can represent puzzles with up to 27 distinct pieces across both colors. The
/// initial state is `(((1 << num_pieces) - 1) & !(1 << player_start)) | (player_start << 27)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct PuzzleState(u32);

impl PuzzleState {
    pub fn initial(p: &Puzzle) -> Self {
        let num_pieces = p.piece_locs.iter().take_while(|z| **z != 0xff).count();
        let to_capture = ((1 << num_pieces) - 1) & !(1 << p.player_start);
        PuzzleState(to_capture | (p.player_start << 27))
    }

    pub fn done(self) -> bool {
        self.remaining_captures() == 0
    }

    fn remaining_captures(self) -> u32 {
        self.0 & 0x07ffffff
    }

    pub fn next_states<F: FnMut(u32, PuzzleState)>(self, p: &Puzzle, mut consume: F) {
        let player_idx = (self.0 >> 27) as usize;
        let start = SquareSet(1 << p.piece_locs[player_idx]);
        let obstacles = p.obstacles;
        let targets = {
            let mut res = 0;
            let mut remaining = self.remaining_captures();
            while remaining != 0 {
                let i = remaining.trailing_zeros();
                // `i` (0..27) is the index of a piece that still needs to be captured
                res |= 1 << p.piece_locs[i as usize];
                remaining &= remaining - 1;
            }
            SquareSet(res)
        };
        let captures = match p.piece_types[player_idx] {
            Some(PieceType::Pawn) => captures::<Pawn>(start, obstacles, targets),
            Some(PieceType::Bishop) => captures::<Bishop>(start, obstacles, targets),
            Some(PieceType::Rook) => captures::<Rook>(start, obstacles, targets),
            Some(PieceType::Monarch) => captures::<Monarch>(start, obstacles, targets),
            Some(PieceType::Knight) => captures::<Knight>(start, obstacles, targets),
            None => panic!("no piece {}", player_idx),
        };

        let mut captures = captures.0;
        while captures != 0 {
            let i = captures.trailing_zeros();
            // `i` (0..64) is the board square of a piece that can be captured
            let piece_idx = u32::from(p.pieces_by_loc[i as usize]);
            let new_captures = self.remaining_captures() & !(1 << piece_idx);
            let new_state = Self(new_captures | (piece_idx << 27));
            consume(piece_idx, new_state);
            captures &= captures - 1;
        }
    }
}

fn solve(p: &Puzzle) -> Option<Vec<u32>> {
    let mut predecessors = HashMap::<PuzzleState, (PuzzleState, u32)>::new();
    let mut frontier: HashSet<PuzzleState> = HashSet::<PuzzleState>::new();
    frontier.insert(PuzzleState::initial(p));
    while !frontier.is_empty() {
        let mut new_frontier = HashSet::new();
        for &prev in frontier.iter() {
            let mut done = None;
            prev.next_states(p, |piece_idx, next| {
                use std::collections::hash_map::Entry::*;
                match predecessors.entry(next) {
                    Occupied(_) => (),
                    Vacant(slot) => {
                        slot.insert((prev, piece_idx));
                        new_frontier.insert(next);
                    }
                }
                if next.done() {
                    done = Some(next);
                }
            });
            if let Some(final_state) = done {
                // unwind
                let mut res = Vec::new();
                let mut current = final_state;
                while let Some(&(prev, piece_idx)) = predecessors.get(&current) {
                    res.push(piece_idx);
                    current = prev;
                }
                res.reverse();
                return Some(res);
            }
        }
        frontier = new_frontier;
    }
    None
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

    let start = SquareSet(0x0000_0010_0000_0000);
    println!("start:\n{}", start.draw());
    println!("knight steps:\n{}", Knight::move_steps(start).draw());
    let start = SquareSet(0x4000_0000_0400_0000);
    println!("start:\n{}", start.draw());
    println!("knight steps:\n{}", Knight::move_steps(start).draw());

    let obstacle_locs = vec![
        (0, 0),
        (0, 4),
        (0, 5),
        (1, 1),
        (1, 3),
        (1, 5),
        (3, 1),
        (3, 3),
        (3, 5),
        (4, 0),
        (4, 4),
        (5, 0),
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 5),
    ];
    let mut obstacles = SquareSet(0);
    for &(y, x) in obstacle_locs.iter() {
        let idx = 8 * y + x;
        obstacles = obstacles | SquareSet(1 << idx);
    }
    for x in 0..8 {
        for y in 0..8 {
            if x >= 6 || y >= 6 {
                let idx = 8 * y + x;
                obstacles = obstacles | SquareSet(1 << idx);
            }
        }
    }
    println!("obstacles:\n{}", obstacles.draw());

    let mut piece_types = [None; 32];
    {
        use PieceType::*;
        let pieces = [
            Pawn, Knight, Pawn, Rook, Knight, Rook, Bishop, Pawn, Pawn, Rook, Knight, Bishop,
        ];
        for (i, pt) in pieces.into_iter().enumerate() {
            piece_types[i] = Some(pt);
        }
    }
    println!("piece types: {:?}", piece_types);

    let mut piece_locs = [0xff; 32];
    let mut pieces_by_loc = [0xff; 64];
    {
        let locs = [
            (0, 1),
            (0, 3),
            (1, 0),
            (1, 4),
            (2, 0),
            (2, 1),
            (2, 2),
            (3, 0),
            (3, 2),
            (4, 1),
            (4, 2),
            (4, 3),
        ];
        for (i, (y, x)) in locs.into_iter().enumerate() {
            let loc = 8 * y + x;
            piece_locs[i] = loc as u8;
            pieces_by_loc[loc] = i as u8;
        }
    }

    println!("piece locs: {:?}", piece_locs);
    println!("pieces by loc: {:?}", pieces_by_loc);

    let player_start = 4;

    let puz = Puzzle {
        obstacles,
        piece_types,
        piece_locs,
        pieces_by_loc,
        player_start,
    };

    println!("solving...");
    let start = std::time::Instant::now();
    let sol = solve(&puz);
    let elapsed = start.elapsed();
    println!("done in {:?}. {:?}", elapsed, sol);
}
