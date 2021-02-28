use std::ops::Add;

use shakmaty::{
    attacks, Bitboard, Board, ByColor, Castles, CastlingMode, CastlingSide, Chess, Color,
    FromSetup, Material, MaterialSide, Move, MoveList, Outcome, PositionErrorKinds, Rank,
    RemainingChecks, Role, Square,
};
use shakmaty::{Position, Setup};

#[derive(Debug)]
pub struct BughousePositionError {
    errors: PositionErrorKinds,
}

#[derive(Clone, Debug, Default)]
pub struct Bughouse {
    chess: Chess,
    pockets: Material,
}

impl Setup for Bughouse {
    fn board(&self) -> &Board {
        self.chess.board()
    }
    fn pockets(&self) -> Option<&Material> {
        Some(&self.pockets)
    }
    fn turn(&self) -> Color {
        self.chess.turn()
    }
    fn castling_rights(&self) -> Bitboard {
        self.chess.castling_rights()
    }
    fn ep_square(&self) -> Option<Square> {
        self.chess.ep_square()
    }
    fn remaining_checks(&self) -> Option<&ByColor<RemainingChecks>> {
        None
    }
    fn halfmoves(&self) -> u32 {
        self.chess.halfmoves()
    }
    fn fullmoves(&self) -> std::num::NonZeroU32 {
        self.chess.fullmoves()
    }
}

impl Bughouse {
    pub fn from_setup(
        setup: &dyn Setup,
        mode: CastlingMode,
    ) -> Result<Bughouse, BughousePositionError> {
        let chess = Chess::from_setup(setup, mode)
            .map_err(|e| BughousePositionError { errors: e.kinds() })?;
        let mut errors: PositionErrorKinds = PositionErrorKinds::empty();

        let pockets = setup.pockets().cloned().unwrap_or_default();
        if pockets
            .count()
            .saturating_add(chess.board().occupied().count())
            > 64
        {
            errors |= PositionErrorKinds::VARIANT;
        } else if pockets.white.kings > 0 || pockets.black.kings > 0 {
            errors |= PositionErrorKinds::TOO_MANY_KINGS;
        }

        if pockets
            .count()
            .saturating_add(chess.board().occupied().count())
            <= 64
            && usize::from(pockets.white.pawns.saturating_add(pockets.black.pawns))
                .saturating_add(chess.board().pawns().count())
                <= 32
        {
            errors &= !PositionErrorKinds::IMPOSSIBLE_MATERIAL;
        }

        if errors != PositionErrorKinds::empty() {
            Err(BughousePositionError { errors })
        } else {
            Ok(Bughouse { chess, pockets })
        }
    }

    fn our_pocket(&self) -> &MaterialSide {
        self.pockets.by_color(self.turn())
    }

    fn our_pocket_mut(&mut self) -> &mut MaterialSide {
        let turn = self.turn();
        self.pockets.by_color_mut(turn)
    }

    fn legal_put_squares(&self) -> Bitboard {
        let checkers = self.checkers();

        if checkers.is_empty() {
            !self.board().occupied()
        } else if let Some(checker) = checkers.single_square() {
            let king = self
                .board()
                .king_of(self.turn())
                .expect("king in crazyhouse");
            attacks::between(checker, king)
        } else {
            Bitboard(0)
        }
    }

    pub fn add_material(mut self, material: Material) -> Self {
        self.pockets = self.pockets.add(material);
        self
    }
}

impl Position for Bughouse {
    fn play_unchecked(&mut self, m: &Move) {
        match *m {
            Move::Normal {
                capture: Some(capture),
                to,
                ..
            } => {
                let capture = if self.board().promoted().contains(to) {
                    Role::Pawn
                } else {
                    capture
                };

                *self.our_pocket_mut().by_role_mut(capture) += 1;
            }
            Move::EnPassant { .. } => {
                self.our_pocket_mut().pawns += 1;
            }
            Move::Put { role, .. } => {
                *self.our_pocket_mut().by_role_mut(role) -= 1;
            }
            _ => {}
        }

        self.chess.play_unchecked(m);
    }

    fn castles(&self) -> &Castles {
        self.chess.castles()
    }

    fn legal_moves(&self) -> MoveList {
        let mut moves = self.chess.legal_moves();

        let pocket = self.our_pocket();
        let targets = self.legal_put_squares();

        for to in targets {
            for &role in &[Role::Knight, Role::Bishop, Role::Rook, Role::Queen] {
                if pocket.by_role(role) > 0 {
                    moves.push(Move::Put { role, to });
                }
            }
        }

        if pocket.pawns > 0 {
            for to in targets & !Bitboard::BACKRANKS {
                moves.push(Move::Put {
                    role: Role::Pawn,
                    to,
                });
            }
        }

        moves
    }

    fn castling_moves(&self, side: CastlingSide) -> MoveList {
        self.chess.castling_moves(side)
    }

    fn en_passant_moves(&self) -> MoveList {
        self.chess.en_passant_moves()
    }

    fn san_candidates(&self, role: Role, to: Square) -> MoveList {
        let mut moves = self.chess.san_candidates(role, to);

        if self.our_pocket().by_role(role) > 0
            && self.legal_put_squares().contains(to)
            && (role != Role::Pawn || !Bitboard::BACKRANKS.contains(to))
        {
            moves.push(Move::Put { role, to });
        }

        moves
    }

    fn is_irreversible(&self, m: &Move) -> bool {
        match *m {
            Move::Castle { .. } => true,
            Move::Normal { role, from, to, .. } => {
                // Whether the side to move can castle either way
                let can_castle = (self.castling_rights()
                    & Bitboard::relative_rank(self.turn(), Rank::First))
                .any();
                self.castling_rights().contains(from)
                    || self.castling_rights().contains(to)
                    || (role == Role::King && can_castle)
            }
            _ => false,
        }
    }

    fn has_insufficient_material(&self, _color: Color) -> bool {
        false
    }

    fn is_variant_end(&self) -> bool {
        false
    }
    fn variant_outcome(&self) -> Option<Outcome> {
        None
    }
}
