// N3485 13.5.3/1: an assignment operator shall be a non-static member function
// (C++23 static operators cover only operator() and operator[]);
// must be rejected
struct A { static A operator=(int); };
