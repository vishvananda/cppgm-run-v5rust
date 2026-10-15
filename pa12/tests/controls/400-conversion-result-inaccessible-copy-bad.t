// Access to a final copy constructor is checked after selecting the converter; expect reject
struct A{A();private:A(const A&);};
struct S{operator A&();};
A f(S&s){return s;}
