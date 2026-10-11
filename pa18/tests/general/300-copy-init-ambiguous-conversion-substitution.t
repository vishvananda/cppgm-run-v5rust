// N3485 8.5, 13.3.1.4: select the conversion, then validate final construction.
struct X;struct A{A(X &);};struct X{operator A();};void consume(A);template<class T>auto probe(int)->decltype(consume(*(T *)0),char{}) ;template<class>long probe(...);int main(){static_assert(sizeof(probe<X>(0))==sizeof(long),"ambiguous copy conversion");return 0;}
