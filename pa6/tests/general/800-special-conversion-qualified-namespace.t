// Frozen types-mode special-member declaration responsibility regression.
namespace N{struct C{operator int();};}N::C::operator int(){return 1;}
