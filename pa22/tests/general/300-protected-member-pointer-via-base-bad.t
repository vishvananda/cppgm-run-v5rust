// N3485 11.4p1: inside derived, &base::m for a protected non-static member
// must name the derived class (&derived::m); &base::m is ill-formed. Must
// reject.
struct base { protected: int m; };
struct derived : base { static int base::*get() { return &base::m; } };
int main() { return 0; }
