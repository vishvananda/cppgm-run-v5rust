// A qualified constructor declaration requires namespace scope.
struct C{C();};struct D{C::C();};
