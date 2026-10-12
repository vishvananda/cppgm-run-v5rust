// Frozen types-mode special-member declaration responsibility regression.
struct C{};C::operator int(){return 1;}
