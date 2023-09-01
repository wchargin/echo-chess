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

// Overloads for basic arithmetic on `SquareSet`s.
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

/// A piece type that moves by zero or more "move steps" followed by exactly one "capture step".
/// This precisely describes the behavior of every chess piece when the piece is allowed to move an
/// unbounded number of times and then must capture, as in Echo Chess.
///
/// (For pieces other than pawns, `move_steps` and `capture_steps` are the same.)
trait Stepper {
    /// If a piece is on one of the given squares, which squares can it move to in one step?
    fn move_steps(from: SquareSet) -> SquareSet;
    /// If a piece is on one of the given squares, which squares can it capture in one step?
    fn capture_steps(from: SquareSet) -> SquareSet;
}

/// Given that a piece of type `S` is on one of the squares in `from`, and may not move onto or
/// through the squares in `obstacles`, which of the `targets` can it capture?
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
        from << 8
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PieceType {
    Pawn,
    Bishop,
    Rook,
    Monarch,
    Knight,
}

/// Concise, solver-friendly description of a puzzle with up to 27 pieces.
///
/// Pieces in this puzzle are indexed from 0 in order of ascending board location, in rank-major
/// order. That is, piece A comes before piece B if it is either on a smaller rank (1 is smallest,
/// 8 is largest), or on the same rank and a smaller file (A is smallest, H is largest).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Puzzle {
    /// Which squares have obstacles?
    obstacles: SquareSet,
    /// Maps piece index (`0..27`) to piece type, or `None` if there is no such piece.
    piece_types: [Option<PieceType>; 32],
    /// Maps piece index (`0..27`) to board square (`0..64`), or `0xff` if there is no such piece.
    piece_locs: [u8; 32],
    /// Maps board square (`0..64`) to piece index (`0..27`), or `0xff` if there is no piece at
    /// that location.
    pieces_by_loc: [u8; 64],
    /// Which piece (`0..27`) is initially controlled by the player?
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
    /// Computes the initial state for a puzzle.
    pub fn initial(p: &Puzzle) -> Self {
        let num_pieces = p.piece_locs.iter().take_while(|z| **z != 0xff).count();
        let to_capture = ((1 << num_pieces) - 1) & !(1 << p.player_start);
        PuzzleState(to_capture | (p.player_start << 27))
    }

    /// Checks whether the player has won: i.e., if all opposing pieces have been captured.
    pub fn done(self) -> bool {
        self.remaining_captures() == 0
    }

    pub fn current_piece_idx(self) -> u32 {
        self.0 >> 27
    }

    fn remaining_captures(self) -> u32 {
        self.0 & 0x07ffffff
    }

    /// Calls `consume(piece_idx, next_state)` for each successor state, where `piece_idx`
    /// (`0..27`) is the index of the piece that can be captured to move to `next_state`.
    pub fn next_states<F: FnMut(PuzzleState)>(self, p: &Puzzle, mut consume: F) {
        let player_idx = self.current_piece_idx() as usize;
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
            consume(new_state);
            captures &= captures - 1;
        }
    }
}

/// Solves a puzzle, returning a list of piece indices to be captured in order to win, or returns
/// `None` if no solution is possible.
fn solve(p: &Puzzle) -> Option<Vec<u32>> {
    let mut predecessors: HashMap<PuzzleState, PuzzleState> = HashMap::new();
    let mut frontier: HashSet<PuzzleState> = HashSet::new();
    let mut new_frontier: HashSet<PuzzleState> = HashSet::new();
    frontier.insert(PuzzleState::initial(p));
    while !frontier.is_empty() {
        for &prev in frontier.iter() {
            let mut done = None;
            prev.next_states(p, |next| {
                use std::collections::hash_map::Entry::*;
                match predecessors.entry(next) {
                    Occupied(_) => (),
                    Vacant(slot) => {
                        slot.insert(prev);
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
                while let Some(&prev) = predecessors.get(&current) {
                    res.push(current.current_piece_idx());
                    current = prev;
                }
                res.reverse();
                return Some(res);
            }
        }
        frontier.clear();
        std::mem::swap(&mut frontier, &mut new_frontier);
    }
    None
}

// Everything below this point is shoddy frontend code :-)

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

impl Puzzle {
    /// Parses "compound FEN" (FEN but `X`/`x` is a boundary), or panics on invalid FEN.
    fn from_compound_fen(fen: &str) -> Puzzle {
        let mut obstacles = SquareSet(0);
        let mut piece_types_by_loc: [Option<PieceType>; 64] = [None; 64];
        let mut player_loc = None;
        let mut y = 7;
        let mut x = 0;
        for c in fen.chars() {
            let loc = (8 * y + x) as usize;
            use PieceType::*;
            match c {
                '/' => {
                    y -= 1;
                    x = 0;
                    continue;
                }
                '0'..='9' => {
                    x += c as u32 - '0' as u32;
                    continue;
                }
                'X' | 'x' => {
                    obstacles = obstacles | SquareSet(1 << loc);
                }
                'P' | 'p' => piece_types_by_loc[loc] = Some(Pawn),
                'B' | 'b' => piece_types_by_loc[loc] = Some(Bishop),
                'R' | 'r' => piece_types_by_loc[loc] = Some(Rook),
                'N' | 'n' => piece_types_by_loc[loc] = Some(Knight),
                'K' | 'k' | 'Q' | 'q' => piece_types_by_loc[loc] = Some(Monarch),
                other => panic!("Unrecognized char in FEN: {:?}", other),
            }
            if matches!(c, 'P' | 'B' | 'R' | 'K' | 'Q' | 'N') {
                player_loc = Some(loc);
            }
            x += 1;
        }
        let player_loc = player_loc.expect("No player location");
        let mut pz = Puzzle {
            obstacles,
            piece_types: [None; 32],
            piece_locs: [0xff; 32],
            pieces_by_loc: [0xff; 64],
            player_start: 0xff,
        };
        let mut piece_idx = 0;
        for (loc, piece_type) in piece_types_by_loc.into_iter().enumerate() {
            let Some(piece_type) = piece_type else {
                continue
            };
            pz.piece_types[piece_idx] = Some(piece_type);
            pz.piece_locs[piece_idx] = loc as u8;
            pz.pieces_by_loc[loc] = piece_idx as u8;
            if loc == player_loc {
                pz.player_start = piece_idx as u32;
            }
            piece_idx += 1;
        }
        pz
    }
}

#[allow(dead_code)]
fn test_steps() {
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
}

fn main() {
    let puz = Puzzle::from_compound_fen(
        "\
        XXXXXXXX/\
        Xxxxx1xX/\
        Xxrnbx1X/\
        Xpxpx1xX/\
        XNrb3X/\
        Xpx1xrxX/\
        Xxp1nxxX/\
        XXXXXXXX\
        ",
    );

    println!("solving...");
    let start = std::time::Instant::now();
    let sol = solve(&puz);
    let elapsed = start.elapsed();
    println!("done in {:?}. {:?}", elapsed, sol);
    if let Some(moves) = sol {
        for (i, &piece_idx) in moves.iter().enumerate() {
            let ty = puz.piece_types[piece_idx as usize].unwrap();
            let loc = puz.piece_locs[piece_idx as usize] as u32;
            let y = loc / 8;
            let x = loc % 8;
            let loc_name = format!("{}{}", char::from_u32(u32::from('a') + x).unwrap(), y + 1);
            println!("{:2}. capture {:?} on {}", i + 1, ty, loc_name);
        }
    }
}
