struct A{int a;};struct B{int b;};struct D:A,B{};D*operator&(D&){return nullptr;}B*f(D&d){return &d;}int main(){D d;return f(d)!=nullptr;}
