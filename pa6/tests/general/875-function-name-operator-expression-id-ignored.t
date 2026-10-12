// Declaration name identity, callable facts and exception specification ownership.
struct C{};int operator+(C,C);void f(C a){operator+(a,a);}
