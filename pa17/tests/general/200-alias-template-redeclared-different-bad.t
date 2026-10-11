// N3485 3.2p1, 14.5.7, 7.1.3p? : an alias template redeclared with a different
// type is a redefinition; expect reject
template<class T> using A = T;
template<class T> using A = T*;
int main() { return 0; }
