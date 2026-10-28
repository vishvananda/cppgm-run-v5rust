struct B{int b;};struct D:virtual B{};
B*ordinary(D*p){return p;}
B*implicit(bool c,D*p,D*q){return c?p:q;}
B*explicit_cast(bool c,D*p,D*q){return static_cast<B*>(c?p:q);}
