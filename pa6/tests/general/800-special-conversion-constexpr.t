// Frozen types-mode special-member declaration responsibility regression.
struct C{constexpr operator int(){return 1;}};
