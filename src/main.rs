#![feature(conservative_impl_trait, plugin)]

#![plugin(clippy)]

extern crate futures;
extern crate futures_cpupool;
extern crate rand;
extern crate time;

use futures::{future, Future};
use futures_cpupool::{CpuPool, CpuFuture};
use rand::Rng;
use std::{convert, fmt};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Color {
    Black,
    White,
}

impl Color {
    fn other(&self) -> Color {
        match *self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    Bishop,
    Empty,
    King,
    Knight,
    Pawn,
    Queen,
    Rook,
}

impl Piece {
    fn value(&self) -> usize {
        match *self {
            Piece::Empty | Piece::King => 0,
            Piece::Bishop | Piece::Knight => 3,
            Piece::Pawn => 1,
            Piece::Queen => 9,
            Piece::Rook => 5,
        }
    }
}

type ColorPiece = (Color, Piece);

const EMPTY: ColorPiece = (Color::White, Piece::Empty);

const FILES: &'static [char] = &['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
const RANKS: &'static [u8] = &[1, 2, 3, 4, 5, 6, 7, 8];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Square {
    file: char,
    rank: u8,
}

impl Square {
    fn new(file: char, rank: u8) -> Self {
        Square {
            file: file,
            rank: rank,
        }
    }

    fn from_indexes(i: usize, j: usize) -> Self {
        Self::new(FILES[i], RANKS[j])
    }

    fn indexes(&self) -> (usize, usize) {
        (FILES.iter().position(|&f| f == self.file).unwrap(),
         RANKS.iter().position(|&r| r == self.rank).unwrap())
    }

    fn in_between(&self, other: &Square) -> Vec<Square> {
        let (si, sj) = self.indexes();
        let (oi, oj) = other.indexes();
        let i_delta = oi as isize - si as isize;
        let j_delta = oj as isize - sj as isize;

        if !(i_delta == 0 || j_delta == 0 || i_delta.abs() == j_delta.abs()) {
            panic!("Invalid in_between squares: {} {}", self, other);
        }

        let i_dir = if i_delta == 0 {
            0
        } else {
            i_delta / i_delta.abs()
        };
        let j_dir = if j_delta == 0 {
            0
        } else {
            j_delta / j_delta.abs()
        };

        let mut nodes = vec![];
        let mut current = *self;
        while let Some(square) = current.neighboor(i_dir, j_dir) {
            if &square == other {
                return nodes;
            }
            nodes.push(square);
            current = square;
        }
        unreachable!()
    }

    fn neighboor(&self, i_delta: isize, j_delta: isize) -> Option<Square> {
        let (i, j) = self.indexes();
        let move_i = i as isize + i_delta;
        let move_j = j as isize + j_delta;
        if 0 <= move_i && move_i <= 7 && 0 <= move_j && move_j <= 7 {
            return Some(Square::from_indexes(move_i as usize, move_j as usize));
        }
        None
    }

    fn left(&self) -> Option<Square> {
        self.neighboor(-1, 0)
    }

    fn right(&self) -> Option<Square> {
        self.neighboor(1, 0)
    }

    fn up(&self) -> Option<Square> {
        self.neighboor(0, 1)
    }

    fn down(&self) -> Option<Square> {
        self.neighboor(0, -1)
    }

    fn up_left(&self) -> Option<Square> {
        self.neighboor(-1, 1)
    }

    fn up_right(&self) -> Option<Square> {
        self.neighboor(1, 1)
    }

    fn down_left(&self) -> Option<Square> {
        self.neighboor(-1, -1)
    }

    fn down_right(&self) -> Option<Square> {
        self.neighboor(1, -1)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.file, self.rank)
    }
}

impl convert::From<(char, u8)> for Square {
    fn from((file, rank): (char, u8)) -> Square {
        Square::new(file, rank)
    }
}

type Move = (Square, Square);

fn available_moves(square: &Square, piece: &ColorPiece) -> Vec<Square> {
    match *piece {
        (_, Piece::Bishop) => {
            let mut current = *square;
            let mut moves = Vec::new();

            while let Some(ul) = current.up_left() {
                moves.push(ul);
                current = ul;
            }

            current = *square;
            while let Some(ur) = current.up_right() {
                moves.push(ur);
                current = ur;
            }

            current = *square;
            while let Some(dl) = current.down_left() {
                moves.push(dl);
                current = dl;
            }

            current = *square;
            while let Some(dr) = current.down_right() {
                moves.push(dr);
                current = dr;
            }

            moves
        }
        (_, Piece::Empty) => vec![],
        (_, Piece::King) => {
            vec![square.left(),
                 square.right(),
                 square.up(),
                 square.down(),
                 square.up_left(),
                 square.up_right(),
                 square.down_left(),
                 square.down_right()]
                .into_iter()
                .filter(|s| s.is_some())
                .map(|s| s.unwrap())
                .collect()
        }
        (_, Piece::Knight) => {
            vec![square.neighboor(2, 1),
                 square.neighboor(2, -1),
                 square.neighboor(-2, 1),
                 square.neighboor(-2, -1),
                 square.neighboor(1, 2),
                 square.neighboor(1, -2),
                 square.neighboor(-1, 2),
                 square.neighboor(-1, -2)]
                .into_iter()
                .filter(|s| s.is_some())
                .map(|s| s.unwrap())
                .collect()
        }
        (color, Piece::Pawn) => {
            match color {
                Color::Black => {
                    let mut moves = vec![square.down(), square.down_left(), square.down_right()];
                    if square.rank == 7 {
                        moves.push(square.neighboor(0, -2));
                    }
                    moves.into_iter().filter(|s| s.is_some()).map(|s| s.unwrap()).collect()
                }
                Color::White => {
                    let mut moves = vec![square.up(), square.up_left(), square.up_right()];
                    if square.rank == 2 {
                        moves.push(square.neighboor(0, 2));
                    }
                    moves.into_iter().filter(|s| s.is_some()).map(|s| s.unwrap()).collect()
                }
            }
        }
        (color, Piece::Queen) => {
            let mut moves = available_moves(square, &(color, Piece::Rook));
            let mut other = available_moves(square, &(color, Piece::Bishop));
            moves.append(&mut other);
            moves
        }
        (_, Piece::Rook) => {
            let mut current = *square;
            let mut moves = Vec::new();

            while let Some(l) = current.left() {
                moves.push(l);
                current = l;
            }

            current = *square;
            while let Some(r) = current.right() {
                moves.push(r);
                current = r;
            }

            current = *square;
            while let Some(u) = current.up() {
                moves.push(u);
                current = u;
            }

            current = *square;
            while let Some(d) = current.down() {
                moves.push(d);
                current = d;
            }

            moves
        }
    }
}

#[derive(Debug)]
enum GameStatus {
    InPlay,
    Finished(Color),
}

#[derive(Clone, Copy, Debug)]
struct Board {
    squares: [[ColorPiece; 8]; 8],
}

impl Board {
    fn new() -> Board {
        let mut board = Board { squares: [[EMPTY; 8]; 8] };
        for color in &[Color::Black, Color::White] {
            board.add_bishops(color);
            board.add_pawns(color);
            board.add_king(color);
            board.add_knights(color);
            board.add_queen(color);
            board.add_rooks(color);
        }
        board
    }

    fn get(&self, square: &Square) -> ColorPiece {
        let (i, j) = square.indexes();
        self.squares[i][j]
    }

    fn score(&self) -> (usize, usize) {
        if let GameStatus::Finished(color) = self.status() {
            return match color {
                Color::Black => (0, 100),
                Color::White => (100, 0),
            };
        }

        self.squares
            .iter()
            .flat_map(|c| c)
            .fold((0, 0), |(w_score, b_score), &(c, p)| {
                match c {
                    Color::Black => (w_score, b_score + p.value()),
                    Color::White => (w_score + p.value(), b_score),
                }
            })
    }

    fn status(&self) -> GameStatus {
        let kings = self.squares
            .iter()
            .flat_map(|c| c)
            .fold((false, false), |(w_has_king, b_has_king), &(c, p)| {
                match c {
                    Color::Black => (w_has_king, b_has_king || p == Piece::King),
                    Color::White => (w_has_king || p == Piece::King, b_has_king),
                }
            });

        match kings {
            (true, true) => GameStatus::InPlay,
            (true, false) => GameStatus::Finished(Color::White),
            (false, true) => GameStatus::Finished(Color::Black),
            (false, false) => panic!("Invalid game state: no kings found"),
        }
    }

    fn legal_moves(&self, color: Color) -> Vec<Move> {
        self.squares
            .iter()
            .enumerate()
            .flat_map(|(i, col)| {
                col.into_iter()
                    .enumerate()
                    .map(|(j, p)| (Square::from_indexes(i, j), *p))
                    .collect::<Vec<(Square, ColorPiece)>>()
            })
            .filter(|&(_, (c, _))| c == color)
            .flat_map(|(square, piece)| {
                available_moves(&square, &piece)
                    .into_iter()
                    .map(|dest| (square, dest))
                    .collect::<Vec<Move>>()
            })
            .filter(|&(from, to)| self.is_legal(&from, &to))
            .collect()
    }

    fn exec_move(&self, from: &Square, to: &Square) -> Board {
        let mut new_state = *self;
        let from_piece = self.get(from);
        new_state.set(*from, EMPTY);
        new_state.set(*to, from_piece);
        new_state
    }

    fn set<S: Into<Square>>(&mut self, square: S, piece: ColorPiece) {
        let (i, j) = square.into().indexes();
        self.squares[i][j] = piece;
    }

    fn is_legal(&self, from: &Square, to: &Square) -> bool {
        let (from_color, from_piece) = self.get(from);
        let (to_color, to_piece) = self.get(to);

        if from_color == to_color && to_piece != Piece::Empty {
            return false;
        }

        if from_piece == Piece::Pawn {
            if from.file == to.file && to_piece != Piece::Empty {
                return false;
            }
            if from.file != to.file && to_piece == Piece::Empty {
                return false;
            }
        }

        if from_piece != Piece::Knight {
            let in_between = from.in_between(to);
            let with_pieces = in_between.iter()
                .filter(|s| {
                    match self.get(s) {
                        (_, Piece::Empty) => false,
                        _ => true,
                    }
                })
                .collect::<Vec<&Square>>();
            if !with_pieces.is_empty() {
                return false;
            }
        }

        true
    }

    fn add_bishops(&mut self, color: &Color) {
        match *color {
            Color::Black => {
                self.set(('c', 8), (Color::Black, Piece::Bishop));
                self.set(('f', 8), (Color::Black, Piece::Bishop));
            }
            Color::White => {
                self.set(('c', 1), (Color::White, Piece::Bishop));
                self.set(('f', 1), (Color::White, Piece::Bishop));
            }
        }
    }

    fn add_king(&mut self, color: &Color) {
        match *color {
            Color::Black => {
                self.set(('e', 8), (Color::Black, Piece::King));
            }
            Color::White => {
                self.set(('e', 1), (Color::White, Piece::King));
            }
        }
    }

    fn add_knights(&mut self, color: &Color) {
        match *color {
            Color::Black => {
                self.set(('b', 8), (Color::Black, Piece::Knight));
                self.set(('g', 8), (Color::Black, Piece::Knight));
            }
            Color::White => {
                self.set(('b', 1), (Color::White, Piece::Knight));
                self.set(('g', 1), (Color::White, Piece::Knight));
            }
        }
    }

    fn add_pawns(&mut self, color: &Color) {
        for file in FILES {
            match *color {
                Color::Black => {
                    self.set((*file, 7), (Color::Black, Piece::Pawn));
                }
                Color::White => {
                    self.set((*file, 2), (Color::White, Piece::Pawn));
                }
            }
        }
    }

    fn add_queen(&mut self, color: &Color) {
        match *color {
            Color::Black => {
                self.set(('d', 8), (Color::Black, Piece::Queen));
            }
            Color::White => {
                self.set(('d', 1), (Color::White, Piece::Queen));
            }
        }
    }

    fn add_rooks(&mut self, color: &Color) {
        match *color {
            Color::Black => {
                self.set(('a', 8), (Color::Black, Piece::Rook));
                self.set(('h', 8), (Color::Black, Piece::Rook));
            }
            Color::White => {
                self.set(('a', 1), (Color::White, Piece::Rook));
                self.set(('h', 1), (Color::White, Piece::Rook));
            }
        }
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "     a  b  c  d  e  f  g  h\n"));
        try!(write!(f, "   -------------------------\n"));
        for (j, rank) in RANKS.iter().enumerate().rev() {
            try!(write!(f, "{} | ", rank));

            for (i, _) in FILES.iter().enumerate() {
                let c = match self.squares[i][j] {
                    (__, Piece::Empty) => ' ',
                    (Color::White, Piece::Bishop) => '♗',
                    (Color::White, Piece::King) => '♔',
                    (Color::White, Piece::Knight) => '♘',
                    (Color::White, Piece::Pawn) => '♙',
                    (Color::White, Piece::Queen) => '♕',
                    (Color::White, Piece::Rook) => '♖',
                    (Color::Black, Piece::Bishop) => '♝',
                    (Color::Black, Piece::King) => '♚',
                    (Color::Black, Piece::Knight) => '♞',
                    (Color::Black, Piece::Pawn) => '♟',
                    (Color::Black, Piece::Queen) => '♛',
                    (Color::Black, Piece::Rook) => '♜',
                };
                try!(write!(f, " {} ", c))
            }
            try!(write!(f, "| {}", rank));
            try!(write!(f, "\n"))
        }
        try!(write!(f, "   -------------------------\n"));
        try!(write!(f, "     a  b  c  d  e  f  g  h\n"));
        Ok(())
    }
}

struct GameTreeNode {
    board: Board,
    turn: Color,
    size: usize,
    children: HashMap<Move, Option<GameTreeNode>>,
}

impl GameTreeNode {
    fn new(board: Board, turn: Color, size: usize) -> GameTreeNode {
        let mut legal_moves = board.legal_moves(turn);
        rand::thread_rng().shuffle(&mut legal_moves);

        GameTreeNode {
            board: board,
            turn: turn,
            size: size,
            children: legal_moves.into_iter().take(size).map(|m| (m, None)).collect(),
        }
    }

    fn size(&self) -> usize {
        let executed = self.children
            .values()
            .filter(|v| v.is_some())
            .map(|v| match *v {
                Some(ref node) => node,
                None => unreachable!(),
            })
            .collect::<Vec<&GameTreeNode>>();
        if executed.is_empty() {
            return 1;
        }

        executed.iter().map(|c| c.size()).fold(0, |acc, size| acc + size)
    }

    fn exec_random_moves(&mut self, depth: usize, pool: Option<&CpuPool>) {
        if let GameStatus::Finished(_) = self.board.status() {
            return;
        }

        let runs = self.size / 2;
        let new_depth = depth - 1;
        let color = self.turn.other();

        if new_depth == 0 {
            return;
        }

        if let Some(pool) = pool {
            let mut futures = vec![];

            for &(from, to) in self.children.keys() {
                let board = self.board;

                let future: CpuFuture<(Move, GameTreeNode), ()> = pool.spawn_fn(move || {
                    let new_state = board.exec_move(&from, &to);
                    let mut node = GameTreeNode::new(new_state, color, runs);
                    node.exec_random_moves(new_depth, None);
                    future::ok(((from, to), node))
                });
                futures.push(future)
            }

            for future in futures {
                match future.wait() {
                    Ok((cmove, node)) => self.children.insert(cmove, Some(node)),
                    Err(_) => panic!("Failed future"),
                };
            }
        } else {
            for (&(from, to), node) in &mut self.children {
                let new_state = self.board.exec_move(&from, &to);
                let mut new_node = GameTreeNode::new(new_state, color, runs);
                new_node.exec_random_moves(new_depth, None);
                *node = Some(new_node)
            }
        }
    }

    fn avg_score(&self, color: Color) -> f64 {
        let executed = self.children
            .values()
            .filter(|v| v.is_some())
            .map(|v| match *v {
                Some(ref node) => node,
                None => unreachable!(),
            })
            .collect::<Vec<&GameTreeNode>>();

        let score = match color {
            Color::Black => self.board.score().1 as f64 - self.board.score().0 as f64,
            Color::White => self.board.score().0 as f64 - self.board.score().1 as f64,
        };

        if executed.is_empty() {
            return score;
        }

        let (sum, count) = executed.iter()
            .map(|node| node.avg_score(color))
            .fold((score, 1), |(sum, count), score| (sum + score, count + 1));

        if count == 0 {
            -100.0
        } else {
            sum as f64 / count as f64
        }
    }
}

fn next_move(board: Board, turn: Color, pool: &CpuPool) -> Option<Move> {
    let mut tree = GameTreeNode::new(board, turn, 64);
    tree.exec_random_moves(5, Some(pool));

    let mut max_avg_score = -1000.0_f64;
    let mut result = None;
    let mut size = 0;

    for (cmove, node) in tree.children {
        match node {
            Some(node) => {
                let avg_score = node.avg_score(turn);
                size += node.size();
                // println!("{} -> {}   {}", cmove.0, cmove.1, avg_score);

                if avg_score > max_avg_score {
                    max_avg_score = avg_score;
                    result = Some(cmove);
                }
            }
            None => continue,
        }
    }

    if let Some(cmove) = result {
        println!("turn: {:?}", turn);
        println!("result: {} -> {}", cmove.0, cmove.1);
        println!("size: {:?}", size);
    }
    result
}

fn main() {
    let pool = CpuPool::new_num_cpus();
    let mut board = Board::new();
    println!("{}", board);

    let start = time::precise_time_ns();
    let mut turn_count = 0;
    let mut turn = Color::White;

    loop {
        turn_count += 1;

        if let Some((from, to)) = next_move(board, turn, &pool) {
            board = board.exec_move(&from, &to);

            // print!("{}[2J", 27 as char);
            println!("{}", board);
            println!("board.score(): {:?}", board.score());
            println!("board.status(): {:?}", board.status());

            if let GameStatus::Finished(_) = board.status() {
                break;
            }
        }
        turn = turn.other();
    }

    let total_time_s = (time::precise_time_ns() - start) as f64 / 1000000000 as f64;
    println!("turns: {:?}", turn_count);
    println!("time (s): {:.*}", 5, total_time_s);
    println!("turns/s: {:.*}", 5, turn_count as f64 / total_time_s);
}
