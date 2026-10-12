// Frozen types-mode special-member declaration responsibility regression.
namespace N{struct C{operator int();};}namespace Q{N::C::operator Bad(){return 1;}}
