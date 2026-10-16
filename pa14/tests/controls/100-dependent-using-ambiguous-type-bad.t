// N3485 7.3.3, 10.2: diagnose ambiguity at the using-declaration; expect reject
struct Left { typedef int type; };
struct Right { typedef char type; };
struct Both : Left, Right {};
template<class T> struct Import : T { using typename T::type; };
Import<Both> value;
