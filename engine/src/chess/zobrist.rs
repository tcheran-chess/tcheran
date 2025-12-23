use crate::chess::{
    game::{CastleRightsSide, Game},
    piece::{Piece, PieceKind},
    player::Player,
    square::Square,
    zobrist::components::ZobristComponent,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ZobristHash(pub u64);

impl ZobristHash {
    pub fn uninit() -> Self {
        Self(0)
    }

    pub fn toggle_piece_on_square(&mut self, square: Square, piece: Piece) {
        self.0 ^= piece_on_square(piece.player, piece.kind, square);
    }

    pub fn toggle_castle_rights(&mut self, player: Player, side: CastleRightsSide) {
        self.0 ^= castle_rights(player, side);
    }

    pub fn toggle_en_passant(&mut self, square: Square) {
        self.0 ^= en_passant(square);
    }

    pub fn toggle_side_to_play(&mut self) {
        self.0 ^= side_to_play();
    }
}

pub fn hash(game: &Game) -> ZobristHash {
    let mut hash = hash_pieces(game, |_| true);

    // Castle rights
    let [white_castle_rights, black_castle_rights] = game.castle_rights;

    // White
    if white_castle_rights.king_side {
        hash.toggle_castle_rights(Player::White, CastleRightsSide::Kingside);
    }

    if white_castle_rights.queen_side {
        hash.toggle_castle_rights(Player::White, CastleRightsSide::Queenside);
    }

    // Black
    if black_castle_rights.king_side {
        hash.toggle_castle_rights(Player::Black, CastleRightsSide::Kingside);
    }

    if black_castle_rights.queen_side {
        hash.toggle_castle_rights(Player::Black, CastleRightsSide::Queenside);
    }

    // En passant
    if let Some(en_passant_target) = game.en_passant_target {
        hash.toggle_en_passant(en_passant_target);
    }

    // Side to play
    if game.player == Player::Black {
        hash.toggle_side_to_play();
    }

    hash
}

pub fn hash_pieces(game: &Game, include_piece_fn: impl Fn(PieceKind) -> bool) -> ZobristHash {
    let mut hash = ZobristHash::uninit();

    for sq in game.board.occupancy() {
        let piece = game.board.piece_guaranteed_at(sq);

        if include_piece_fn(piece.kind) {
            hash.toggle_piece_on_square(sq, piece);
        }
    }

    hash
}

fn piece_on_square(player: Player, piece: PieceKind, square: Square) -> ZobristComponent {
    components::PIECE_SQUARE[player][square][piece]
}

fn castle_rights(player: Player, side: CastleRightsSide) -> ZobristComponent {
    components::CASTLING[player][side]
}

fn en_passant(square: Square) -> ZobristComponent {
    components::EN_PASSANT_SQUARE[square]
}

fn side_to_play() -> ZobristComponent {
    components::SIDE_TO_PLAY
}

#[expect(clippy::unreadable_literal, reason = "Zobrist components are not supposed to be human readable")]
#[rustfmt::skip]
mod components {
    use super::*;

    pub type ZobristComponent = u64;

    pub const PIECE_SQUARE: [[[ZobristComponent; PieceKind::N]; Square::N]; Player::N] = [
        [
            [0xbb2a3fb2cd2c6f7f, 0xc6017c948e27697b, 0x069dc102cf310a16, 0x958b761dabe5f6d0, 0x431d9d54dee17b11, 0xc5a0ef111f71c422],
            [0x37fc854f12037913, 0xcb30ce1ac9ff61c7, 0xbfd4a4ae9e0d7fac, 0xf80c4de387b83854, 0xff0ea77dd9987f7e, 0x23ae2c7b48501800],
            [0x1ce4b87b0bd4b7bb, 0xf6ff78effd960655, 0x0ca57b6234bb13f0, 0x6cfacf846e3bd6a2, 0x75e88c63a6e1329a, 0xec9c7a3c30f0a328],
            [0x3a1f55f0fed54eba, 0x8f3066bc65781cfd, 0x27c7951faf976aeb, 0xd5e34c79b892a064, 0x345f099776ef4fb1, 0x80cd14a8135f3ef3],
            [0xa3438bd0e15e4e8d, 0x7a95e009bf5704d8, 0xe04696e7582b922f, 0xdee3997ccb29252a, 0x102ff028bb620156, 0xfca4dc38ecdca315],
            [0x37800b8b295b5373, 0xfa202be26fdc7e07, 0xeadd98ee4c0bcc72, 0xad5d35116362a0a5, 0x03d8ae10610e6994, 0x11b8823ad192ea97],
            [0x9e3f6128db1dfde3, 0x9d1ffa92b36998ad, 0xc9055662abf1be91, 0xaa77ac12532fc768, 0x159ba8dbdd58898e, 0x157eb310d0efd947],
            [0x84ca4d1af46bbc6e, 0xc0c08aa8dc6b7cd6, 0xde2674d5410142c3, 0x166d45842a6f3545, 0x59e3238a3c59c49b, 0x2ff64e1be5db98b8],
            [0x85bec518e299eb10, 0xfa077d269f85f25d, 0x4389963b69194a02, 0x400a21106742ac36, 0xfc2699b77bb0adce, 0xaa7fd3f5ffd75ad9],
            [0xac0c944918dfd27d, 0xcab2a0b39fa708d2, 0x4ae55c63b250cb87, 0xa26c9c2bcd8257a1, 0x08c8f3bdea01252b, 0x15661793eab81afa],
            [0x318ce55a03fbe027, 0xa7e71c18bf4da8ff, 0x94da8bfbbe1bb62a, 0x2fcfbadc0f7c0b95, 0x375b2523ddf70276, 0x51909fe2ba649cea],
            [0x5a6323524d959578, 0x8e89bfb99d62d540, 0xa26dcf123267ea3d, 0x049f3b6a737b27b1, 0x154b75d893b92b66, 0x52e8b61030a0e916],
            [0x3eef7ddcbb23597c, 0x09edd7890b40708c, 0xea577d10980b1fc7, 0x711f22ff419c038f, 0x9c2b1c9528385520, 0x0c30578d6e53e9e1],
            [0xcd07e4ef3176d3df, 0xd05413868bd2d489, 0xd7595f18596cfbb1, 0xea776055cf8b53ea, 0xc3fe4c62f58e5a61, 0xe135c8a60fd0104d],
            [0x5a231b5305f6e2cc, 0xe40ce9aef3bd5e7b, 0x8a88bf6e2659f3d5, 0x1b588a2ff2c1ca60, 0x49876c8c8ce59693, 0x5c76d14edbc6d480],
            [0xc53885f8bcaa0b27, 0xdb8bc1960b38a131, 0x2c85a30c76ea4d61, 0x83ffd628568a3988, 0x32e5c923a4dcb4c7, 0x1b6e1a9866a817f7],
            [0x4823a14f7b440250, 0x52ce93a4504746b2, 0x4806d461fbdad444, 0x4fb9360bc983decc, 0xd7fc04b3ee7f88eb, 0xdf1929284d0ec625],
            [0x7f8ad0d91143f20e, 0x8e71a00ab6b43013, 0xc62763bb2fdf0759, 0x857290313bd545db, 0x14492a011e97491c, 0xfa6bbf736ac017fd],
            [0xc1e1c80988a3d149, 0x3664d41f37e1eaa1, 0xb889b148273ce79e, 0xfc8a252849527f66, 0x2c242a3b519e9eb4, 0x40d8ec3d8dd73336],
            [0x58421aae970ff281, 0x48983e68476e1c8b, 0xe2773246a3fe0999, 0x9946bde57da149ec, 0x3369a1822cbc13c6, 0x89654f597a3488cd],
            [0x7f6145654365c98a, 0x20d705bfc84349ea, 0xd2c0ba3a6ab4aff6, 0x4fc6bd72ab4a8935, 0xc96c35e4066d4bdf, 0xa6fdacb5c75f8e5f],
            [0xe1feeee502fd55ad, 0x947accdc6545398b, 0x7357ca939d1b14c3, 0x371c60abee87013f, 0x6184bcd041e28120, 0x8eb145c678cf7c2e],
            [0x9df049c2d49e40d8, 0x405d3f126e371f62, 0xfaaaddeb61114a48, 0x1155d7e0588fdd3b, 0x0e39bbd444d2da4d, 0xd60fbb1c87f73a64],
            [0x1d57d5067d1931c4, 0x457e7201e0d0fe2e, 0x0775cb571e1c8451, 0xa72528e929a4223f, 0x06aad29db9782ec6, 0x0796459b88d00ef2],
            [0x65f598090e57a183, 0x38b5389946c3b387, 0x5680dc8590756e09, 0xe0253871d6c119bd, 0x3f899bb45f32b148, 0xb58e8280adfd5413],
            [0x24229b92a21606fd, 0xa9142bc3cfcae5a7, 0xa2e758d174d7cc3e, 0xddf17ac9d0316f86, 0x0054628b7cb314a2, 0x5da13e2fae5de490],
            [0xbd64afbc6088b870, 0x0a0d39942e201f44, 0xa97aa0016f146ddf, 0x4e238b8dc6a145f0, 0x91c1391e71debacb, 0x87d478812d451f15],
            [0xf3646c2b8a272803, 0xa6f7a82cf51d1785, 0x160781092d806554, 0xb340f0c6b084c2d6, 0xe958e53c27404b81, 0xafd899d1cff8d249],
            [0x230e219c41aefd37, 0x9d2922c1a48228bb, 0xda244b689551c4e9, 0xa250c46e30c45281, 0x7be9485c58150107, 0x8b45b05c07d804c1],
            [0x80604602e3894f12, 0x164f75eb84cbbfd7, 0x6d5df77cbfc1a127, 0x7d3743b1998adf27, 0x914067778c6be458, 0x9d0d89f221ee2d51],
            [0x23187b20bc9062af, 0x0b4af2be723bc001, 0x72ac3f12c7e55437, 0x19c1730d435e86ef, 0xa9cfd07b195d6bdb, 0x1636f6bcf40b2579],
            [0x2289825abdaad458, 0xa71d99fccc852640, 0xed5030589e89b1af, 0x57c46375b54fd9bf, 0x96cbdd17d2fd4900, 0x946f0435edd197e1],
            [0x0d3d526602848591, 0xb259d7f9223add84, 0x0e37cb8a40efc6a5, 0x005c532a6e24903a, 0x1a9200f866c082e5, 0xbe83c667d1175d64],
            [0x25752599eecb1c03, 0xec47bd0d502dcb43, 0xe21aa67138a3e416, 0x29163d1adde793c3, 0x6946a131018b7b24, 0xd4be3b36cc0e0c18],
            [0x2d5d0cb8cac18325, 0xb8fa4a7ed9dd705d, 0xaafa98fb13d302f7, 0x8b828615c764a7f0, 0xfa60eeb02224bd33, 0xe50e402f3d2f9390],
            [0xdfa5474ddc961ad3, 0x6f9a8ece10580e3a, 0x2b3ce1494ca5a3c3, 0x30dd91faf3efde12, 0xe63262022b8ba210, 0x108dc88eb56be2cf],
            [0xcceb9cfaa3a3d5f6, 0xb12436600314a052, 0xd4d24b8c9ebe996d, 0x4939d4759e61393a, 0x9679d3b59e3f8769, 0x196a60679eab9d29],
            [0x8314826fedd29f21, 0x7aecb13f91986a8f, 0x4e6e55a1a067e4f5, 0xf8608973b61d31fa, 0x0aac43046b4e6ee5, 0x7b9c6715347cfd63],
            [0x46b95893e752af20, 0x33c4c445c2de38ec, 0x6c797b9bcbaf2b9c, 0x07829602a5bf25e9, 0x39bbd9f6212b6fcd, 0x361da7edce44106a],
            [0x3d40a9c8e7f3505c, 0x8153d1a16f1b3bbc, 0x306ceb8e890013f4, 0xaa95f5ba03484f60, 0xf71a3287ba3a43d6, 0x57a159a823319c98],
            [0x7af6cad3a3f50528, 0xf965d4d5a195ccef, 0x68cbf58a4a42a625, 0x69705fa77f629288, 0x8a423617c353bf5c, 0x4048fca561942c9e],
            [0x667aa775dcdf01c6, 0x98e55201f3333d9f, 0xa8e2da9b1931e33f, 0xf65719b9a46fc9c0, 0x5ffa332cefc40edd, 0x27870e911b744a34],
            [0x55aca3c845285b5c, 0x2e1c69cad63fa9dc, 0x780054b44e4bcb39, 0x8dee92f885ccfb69, 0xa3cb1a3864a2140e, 0x31a740145a2c0722],
            [0x76891bf13ad1c1b5, 0x9ddda6d1397d0e57, 0x3601a5d35a7da5c7, 0xb2def8203512766a, 0x6c37dbb99e1fc358, 0xa27e3624517a5dc7],
            [0x4b5a1709b7946f5b, 0x63c6beef910efed5, 0xd302795e38a7ce5b, 0x147f04b6bb81d01b, 0x49032a9b02f027eb, 0x6639879ba97ecf82],
            [0x667e17b6bece9610, 0x55c4a09f99d0b5a2, 0x070a26042d992ba5, 0x6c7626e229192d16, 0xa87ce8a0b8d5ba76, 0xc5c00a9c8391c8e7],
            [0x68001466069d07be, 0xb64dcf4eb1516753, 0x817c8971e93262d1, 0x8d4ec040fe688f3a, 0xff6895a25ad2ce46, 0xb979fe45c3b09f7a],
            [0x80344880b1147d6b, 0xbd43ada2f3ed8cba, 0xbcc0408074c5b9a3, 0x4343a9ed21d9bcd5, 0x0a7ba15affd269f1, 0x08b468af7a73ca5d],
            [0xc8d05944d9ff9019, 0xfa55a695f5588fc3, 0x38435771d5dfdc60, 0xae90db6e50d54dee, 0x4710f2c85b8aac80, 0x1adb2ef216cfff56],
            [0x7dbf68446d4b2007, 0xf4a1eee51c073ea8, 0x57dc93ca319eb6d6, 0x406b9e002dffcc27, 0x34b48e89418f9b6b, 0x37d99e04042b542b],
            [0xe2e643fc84a11af6, 0x6166ab795316a18b, 0x1ccc7b6eb3ee7439, 0x71a2989794154127, 0x3e8008181fd1cdf6, 0xdd6aafaf71822877],
            [0x4239587f8e9cd44e, 0xe93fa913882d8683, 0xb798db82e68727ad, 0x42d1403012ffde34, 0xc5faf350970dba29, 0x882b5c31183c68b3],
            [0x9381ccd5615f6b84, 0xf85c5297f2981bab, 0x0423dc7b5469bdd1, 0xc01d3738b0b81d4c, 0x332d1841b61ba6ab, 0x5f8865b0b4ba5ea2],
            [0x0bc932bba0270a65, 0x2ad9bcc08746cdd0, 0x40c1e9ffb8ef3d35, 0x2972a1ce539f81c1, 0xba8aee7bc296061c, 0xbd07e8f97de78ffd],
            [0x9f78a3e11971e0ac, 0x9b7a329360996ebc, 0x1724cb8b46e2866b, 0xb8cc18c239a45d1d, 0xcaf21212882dd476, 0x39c6924ab2c69e7f],
            [0x1c28456aad113d2f, 0xc41bd9fbcfcea484, 0x416ef849a6797f27, 0xda26cf7839f582ea, 0x65003ae5255a2700, 0x0a1ba7519c52c99d],
            [0xce02bb311ddd6c90, 0xb537cf824676cc97, 0x83d5942ee2db846d, 0xa67041df8cefa874, 0xc293eb207efe7ce0, 0x21f8f48f4a840c8e],
            [0x60c8f5c6bab1a797, 0x6f598f277a2cd793, 0x68ab39506cc40a01, 0x4e9668fb2d0b2f66, 0x023de7f8a9e10096, 0x677532e5349356d1],
            [0x7d9e971e7ca7f4be, 0x59389aa9d062bc03, 0x0a99c9adb5d7dbea, 0x0ae5b3b69f543bc3, 0x22425fffacce4281, 0xbc8c316b78f12b11],
            [0x3bf2b65bfbf12cae, 0x01e25bf65fbfcb35, 0x74da2dda9ca31236, 0x3475c777ab0850e1, 0x82a7c7303b4e5e08, 0xc8edb3cf6475d108],
            [0x8f40a775a68bbe90, 0xcb77790c3a145ff0, 0xcae44f8d91a418fa, 0xe7af273f4e7c7d46, 0x892b8f13e2af54c4, 0xf18a8741ec0ca100],
            [0x6ee10ee4f95301b1, 0xd9b16ba7bac43e4c, 0x8beeb3abfba59915, 0x15c5b48c1369d7de, 0x374a882e8cde065d, 0x4fd84450d33d8313],
            [0x786fba900a364653, 0x2a156d134928e34b, 0x8f793f68a4d18cdb, 0xa50ced53d2f5dc5a, 0xec591020bac19c5e, 0x8a53c5a09612eda2],
            [0xf5c64011232a94e2, 0x39aa64b582fea2f9, 0x14fa61684c1b5870, 0x0e29ece90573c760, 0x90a1804034f33d30, 0x2c0f73b51555391e],
        ],
        [
            [0xbc0005f4cbe5d6ba, 0x3f66b01b7f544254, 0xbc6321896b1a2498, 0x19c211d2e1e6de09, 0x545b6d0508bad792, 0x119c560c93c53356],
            [0x1ab62034ac0d5557, 0x7c8e4c1b98f22d00, 0x4a5a3fac9a8ea3cb, 0xd4e79239df3be7c2, 0x23141fa4055a6690, 0x32e00c63c1de7435],
            [0x3fb952d8dbcb3c10, 0x27d47413b5668bdf, 0xf6760353bd8f3827, 0xc07a1d930ae8171e, 0xea8ca336bbf0c187, 0x43413ed9b34eb7c7],
            [0xa1719cbfa0e6d0e5, 0x107cbf12cc491911, 0x364b9b05e6a263f2, 0xb40ba35927eeec1a, 0x7ca011045137adf3, 0x2886dc9f702b4d1e],
            [0x705882b2ac9a34fd, 0x5113556e2f25d741, 0xfbb5ce00d58102a2, 0xac30a53a6010d59d, 0x55933c4cda22646a, 0x03eb62165ca7c257],
            [0xc8d2a3ae5dea8c94, 0xfad4b0b60c8fcf71, 0xf256df4efead72e2, 0x163bbb5526df7663, 0xee053d395b3c1c61, 0x50bd909fba0488fb],
            [0x8c49e1e06ab4dae2, 0xa3a2f764dc93549b, 0x29d43a41c048aa91, 0xd6f2186c0c36ce93, 0x7dabbc8a5acf0a6c, 0x27ad77471cf7fc8c],
            [0x62faa075da97a44a, 0xdf2312a4b31cb0ae, 0x279998665cc4f0ff, 0xf9c8b369a37a1b54, 0x9d07c27ce4306cbd, 0x63191d6f43643b09],
            [0xe22dfecf4d56ee4d, 0x33c3460f92ed6c40, 0xb2751ccedc00f4fd, 0xede330754f16d7b0, 0xd8739031229f75ef, 0x98c399a314ad29fc],
            [0x49a14a271ecdbab4, 0x471520667de452a6, 0x0d9d82b5f90241a1, 0x3dd1a2ef119f8ff9, 0xc51c1657551e4d9b, 0xb084a16f9c110f3d],
            [0x1774a16bc833c894, 0xfc3d953b286798fc, 0x4ead4a10db265a03, 0x960bcf96dfaff486, 0x5b8e32a13f8ba696, 0xeb0726bf0fe028f5],
            [0xd023d212aa49e4c4, 0x0c0a06dd02c76c35, 0xea159503d9d63f6c, 0xabac63f80614536a, 0xe6928cf1fe9a7d91, 0x63ff7ed9b2b11d66],
            [0x2637f5cebd4095f0, 0x1ec72c397c8cef89, 0x7a251b4bffd3b470, 0x77af7c923e3b7c8e, 0xb84038f730f23545, 0xe14987276c74c15c],
            [0x96cd16100b5e0d7c, 0xf077049e45613c02, 0x5658ac073943b517, 0x0b8c9a4abdb92143, 0x67930efe7d68c533, 0x32e6565c4cd77029],
            [0xbfb31ba7d169a8ae, 0xd8a7ab3f38afb0d1, 0xa7dbbb8f4f707bf6, 0x143e0e88110f4e08, 0x1617de5ab46efac5, 0xe3e0bd781b662714],
            [0xbeb712e375d7882c, 0xb77be6c353d47eea, 0x016b64844b392e93, 0x295fd1f41ab5dcdf, 0x284dd94f01bee2f9, 0x4df55c176a78cb5a],
            [0x8b29d57355cf96fa, 0x1caf6e28276e381d, 0xc55c316385661f47, 0x8455bcd99ccd46e7, 0x72e113759a3adca3, 0xc0b6697356509281],
            [0x6e42f01b5cac7a42, 0x8c28d091f5845383, 0x7f1954204778da07, 0xbd1b8e4032439eae, 0xfa787504ddb7a25e, 0x197b67d3125e3896],
            [0xdca3804469ccd9d9, 0x84fc766cc4166702, 0x1add4781e90db0e9, 0x48a7d27c6fb73bb3, 0xd815d68ef6a07079, 0xaed1bef688fb3fb4],
            [0x3818ff16731b4004, 0xccf9e605d6d47686, 0x055824db9097d2b9, 0xbe6510c68dd67902, 0x893758c150dce340, 0x0f2d950dd7b3a680],
            [0xc7775d11f4693a1c, 0xc731cc47d42312a1, 0x545bf536992483e5, 0x8ae586aa2f1209b8, 0xbde22e7959feb690, 0x1ed381c7e1777307],
            [0x4866838232e87290, 0xb1e3d77f1be82985, 0xbdab4d6e5ccbfcd6, 0x592b292554b429f0, 0x363ecba9df478bbb, 0x7237ee1a438f8b9b],
            [0x70e304c52df7d680, 0x9ba1ea76a2fb03d9, 0xf39a44b452f95771, 0x0f6d7b8a1858eb15, 0xeb5d5e3dec6dc79f, 0xc9a1ed27d5f00269],
            [0x5935800621661a5f, 0x4c7c34917daaa1ec, 0xc52d31b886666497, 0xdb3173293d3a2f45, 0xf2f69d389e2f6660, 0x0a957703340610e7],
            [0x197afa6bcf479aeb, 0xdb4236f2375da04d, 0x8d42125ab6196cfa, 0xe6817e331e52d071, 0xa1b39c77af2da46b, 0x6291b34e392f8aea],
            [0x5e9d68cae0d00682, 0xa78045169058860f, 0x867522be07766ec5, 0xc3afde177eaa2b16, 0xc7f635533f8b87f5, 0x9836234021143c5c],
            [0xfa5c01a3c8545ffa, 0x7a86067919511a6c, 0xc3e918aa87893904, 0xa648a33cc6cc3e69, 0x83e7f1bd22638420, 0xfd278f04cb212126],
            [0xecc3cee5e4459ed4, 0x473defb8d00cb518, 0x67fc96844f4f94cb, 0x32eb8a11e89aeaef, 0xbd39b2acd3bd60c8, 0x91d2a6e5a0249fbe],
            [0x3ed4ecaefc52ab42, 0x7b79589e505a5f5f, 0x4f0b01d3846fcac7, 0xc8dab5f446bfe950, 0x2327197ce1dcf974, 0x75dba58d57de84b8],
            [0xfbf77ba52a2008e0, 0x429d0cd699758d1d, 0x66abdfabafe3ffba, 0xa7088419a62745e3, 0x004788c6a845a6d0, 0xb7f543cc556f7aad],
            [0x798bd3aafff40383, 0x2b7019ba87ec087f, 0x73ddb9484c2c6bd8, 0xe1efa9ad054f2a66, 0x8020eb693ec5d1ce, 0xe3f1e57f7545dc46],
            [0x940cc0142ae25e68, 0xbd96e3dc6163b841, 0x6f1bc747897a6b6c, 0x74c49b8a806aeefd, 0xb319bbb67c2101de, 0x383d1636cac597c2],
            [0xe683aff5a8da2b54, 0xab10265beedf596f, 0x8dd42fa3d57ba991, 0xc1fc2b31ca87158c, 0x80829c4c17f26c05, 0x3ced22875a059d55],
            [0x29a8d8685a7cc997, 0x90c2f00124bc08d5, 0xdbf3fee65c3f7fad, 0x38b637110c413a13, 0xd08adaed885524a8, 0x33d3af9e010e0d6e],
            [0xdef6b7338c44f7b4, 0x66f5e09b886984fa, 0x19e27319b3d45f06, 0x839bb3c1b6ed7962, 0x55bfc4f924399182, 0x16b2d42c210079c8],
            [0xcd65ef13f236b58b, 0xcb0b33b257a06441, 0xf8a712f7d83120a6, 0x8be1d2d57a8aa03c, 0xebcb77df4e6eef8e, 0x7cafa74a58f848d8],
            [0xc130510d46e268a0, 0x26548724a057c454, 0x7499451197c4fce7, 0x7673e8df487c4241, 0x5fe28dc5010d9799, 0xf8c6ad729b8d225e],
            [0x9f98d7556dcd6a8c, 0x16bdf0a32dad77fb, 0x220689a3b2a0d63a, 0x35ad7acf5b78c886, 0x93727c9ca32d6293, 0x2319ddd6ddb4da9c],
            [0x59ce08992cf46532, 0x74b32d736b952366, 0x67818513e81d03ab, 0xc4f596b6623ce8b4, 0x6f3ad587060aace2, 0x2c84a96779554a51],
            [0xfb75f0f26400a7f7, 0x8bd6f3bb0abce2de, 0x370207dc8e7f9211, 0xfe10d6d932ae71d1, 0x97c60ed6155f6f47, 0xfba3ff8691587c58],
            [0x9002accb8c845c6c, 0xa127f3460582b730, 0x15c101cf877952c1, 0x7b663ca4c2ec0921, 0x045e6b90450c0341, 0x1295bfdaabe17100],
            [0xe5869b97dcc69e5d, 0x05d4879259e2c939, 0x7ec5ee6c66b35f39, 0xa36a267516e4f39b, 0x51f247b40966cccd, 0x9cac751794c86e0b],
            [0x6020790da4efb7e8, 0x19241cfc9a0e66c1, 0xe1db779cee4e091c, 0xc68d7c9d77b1bdbe, 0x65c30c79ad86409a, 0xea2f90bd3f76fe4d],
            [0x21463caa7d2ff0ca, 0xefce1aa79451c11f, 0xf39162ce34280c2b, 0x83b520a968983b99, 0xddd8db34b2e12cb4, 0x3db1e4d67d134be0],
            [0x7edec8899651637d, 0xb0f9a4258d70e667, 0xee3e151f3263462a, 0xd36de07068ed8a6d, 0xd59b041479aa307a, 0x28e7f1130ca53b33],
            [0x0ee9154a6b5d8259, 0xdfa259b4cfbfea68, 0x4f1a89716ecdb134, 0xb73e6cab8e997a2b, 0xf3921c1d47a83b9e, 0x6cd4ad0339d5c6f9],
            [0x2ae1558e297b5df8, 0xd68caedd17cfacdc, 0xa1118c59188e2a4d, 0xc6b67edcff4cfdfe, 0x998c9f41b6f6d6ae, 0x4ac4ef7693727023],
            [0xdc8e08adec7643c6, 0x6f1e4e71b096031a, 0xc3e81888882317a1, 0x7dcdc0486fdf274d, 0xa95d6fa37851ef74, 0xe37d44471f63276f],
            [0x5795b38921452c09, 0x825b5a8e7bc4c20f, 0xb40d802fbda60c42, 0xd0886138ca9aa99c, 0xf293f35c13a7223b, 0xb9d7f074bb260d86],
            [0x5eac48e524d6ce13, 0xa7b93ce0c6800978, 0x0231ea04079a1e52, 0x2b596b4efd1c9a99, 0x556a15743b5b8fda, 0x07283068442e3db8],
            [0x8cc57fa9020518b4, 0xd3d725daeed2c8c0, 0x354b202c912fda47, 0x92376c7b4fb12808, 0x11b96bf0ea24e2da, 0xbdb922c4031f0617],
            [0x2def9b2dfdb304fd, 0xacd7aa31f3e76caf, 0xb11ed2162a2db702, 0xa065387427feb687, 0x5ef42a1ea1e58bc1, 0x3beab6a04a8db95e],
            [0x0a69031392ba9d1d, 0xbb59f20833b75600, 0x32f7d780de735342, 0xe190ab9ca4bef86a, 0x02da30b321619a4a, 0x04b270ae62ee6103],
            [0x71381a75c3e3a2a0, 0xa5ab9b77e332815c, 0x6f1638afacfd28c2, 0xfa756dad56bfeaa8, 0x9103cb172c11bd1c, 0xfb24b0371030efa6],
            [0x6455399cf54cd1f9, 0x85b710f73f305096, 0x6613e41dfa4efcc0, 0x799a3a4a0b29a7b6, 0xe2544bc19f7ce8b1, 0x9ebf0755e36bd24c],
            [0x500238cb62137099, 0x807243ec7b718715, 0x418cd2f078e42316, 0xeeb12e3112fd383e, 0xbdcd85bacb79dff9, 0xab93a4a6df23d492],
            [0xdc557c8dc135610e, 0xfb764d7ceb77e1ca, 0x3ec9480060c8155e, 0xe18b2f31c1fe6bb1, 0x7a114091baae4f74, 0x618c511592e7daf4],
            [0xb6e9479c690a688b, 0x2ff2d28a1889964a, 0xd3c605cf9c62f372, 0x6bc9f71feba56f0b, 0x93f461913c24f14d, 0xbeac8e502db5768f],
            [0xb9478671c21724ea, 0x2e11d71d17bcfc47, 0x1a84f166bd5bc1ac, 0x3fb62c222cb960cc, 0x2b058968caafd77d, 0x365b6767608190d9],
            [0x8d03e926e5c52659, 0x3f6b81489cef34c3, 0x5f84c5c9c61d13b4, 0xe362c806debef1ff, 0x705614240d53af3b, 0xfbf93b15788913b3],
            [0xef67c9548eb0011c, 0x9068a6acdca68eb3, 0x0255bd0ae72ce1e1, 0xa487f67bfbe56579, 0xb6cb13e3e532db84, 0xca6443b890953c18],
            [0xb79aaad34e2b5278, 0xeb2c8aefcea3450b, 0xe418c72d156d9172, 0x1279032ea0ec2b42, 0xfe5d268cc140a59b, 0x0cafa7ad87886ce9],
            [0x50a3f720bd68e091, 0x437ab67f6321dce7, 0x096b054f407e11ed, 0x1d51bc6edface1cf, 0x00aa99df693ea128, 0xa668816b06d3d674],
            [0xf07c2a6886e1bf0d, 0xf20a4bfd311b8fa4, 0x159feeb592e7660a, 0x35283e1ab6386031, 0xed0fe6b9c99aad26, 0xb3bd179d914c3ce1],
        ],
    ];

    pub const CASTLING: [[ZobristComponent; CastleRightsSide::N]; Player::N] = [
        [0x44a6fec3f9626972, 0x5ad7af770c130017],
        [0xb8b25d2d838d7f57, 0x6f048935cc2a947a],
    ];

    pub const EN_PASSANT_SQUARE: [ZobristComponent; Square::N] = [
        0xf304e8645269db0a,
        0x42f12db55aa73666,
        0x12ebea10a351c6ae,
        0x2e33ee4bfdcadb1f,
        0xbb4b25fc1c8d0e16,
        0x193bb192acc9aaee,
        0xd311e973c449a52c,
        0xb3c8c7adfb4ab891,
        0x46ca5d04716f4633,
        0x7973dec1a4bf8d43,
        0xf8b418e7e4c0f96a,
        0xd5c471d5b57f737e,
        0x8cb2d836bacefa8a,
        0x042b916579126a6c,
        0x5639e18fde1998ec,
        0x48284ca715707073,
        0x66643fc6e6d2c7b6,
        0xe3144bf9a25d8fa1,
        0xec299fedcfe21f9e,
        0xdc533bc7f11bcdcd,
        0x2268a75272ccf428,
        0x77c425c41efc4692,
        0x1d1ed92042432081,
        0x25d1f8b96b6b4c7f,
        0xc9af94779a2ac142,
        0xac8a3215918ce313,
        0xdcc3bc00d04af903,
        0x093d490d7956bc70,
        0x18367ca4a5267d70,
        0x85f65ae546e7ab42,
        0xa16a2e25e80a47a2,
        0xd00cd1c3b755a4f9,
        0x67c659ed1d764bee,
        0x932ff6bd3beb8a8b,
        0x619eef4493d305e3,
        0x8a43c3357889fec8,
        0xc8750df3d76a61d8,
        0x460da77181be44ad,
        0x7e18b34e7ca7b6ef,
        0x5838acb2846480df,
        0x45d369b20ce13e4d,
        0xaf8e37bde20ee30b,
        0xe1518807cd75caf1,
        0x34815baf71611d2e,
        0x22516b76bd87490e,
        0x32b2765ffa0918ba,
        0xa1307b2218e13da5,
        0x52e6c158a7139a07,
        0xd59c5d53e268b0eb,
        0xbc0fa9bfac18eae4,
        0x8edd90dde65b2d3a,
        0x373543eac6c8f965,
        0x62309b3ce39237b8,
        0x08eba661968c1252,
        0x531c4a96f3590339,
        0x40a7df0479b9ad0f,
        0xee6e6efcd8e9b34b,
        0x0bcf90c14367770e,
        0xb2eab55cac2c9e1c,
        0xa60fbeb3d167bf4a,
        0x245b440eecf3f788,
        0xd32ea15612a778a7,
        0x7b5e2f88de756f4f,
        0x1df12a2a544aed57,
    ];

    pub const SIDE_TO_PLAY: ZobristComponent = 0x7c7784b34beee83b;
}
