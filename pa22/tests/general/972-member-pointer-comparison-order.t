struct A{int a;};struct B{int b;};struct D:A,B{};int D::*rhs();bool f(int B::*p){return p==rhs();}
