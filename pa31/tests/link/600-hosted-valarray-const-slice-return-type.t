// N3485 26.6.2.5, 13.3.1: indexing a const valarray<double> by a slice selects
// the const operator[](slice), whose return type is libstdc++'s _Expr;
// instantiating it is valid
