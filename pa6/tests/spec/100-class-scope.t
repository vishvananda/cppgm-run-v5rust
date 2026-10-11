// N3485 focus: 3.3.7 [basic.scope.class] class scope
struct C { typedef int Y; Y y; int f(Y x) { Y z; } };
C c;
